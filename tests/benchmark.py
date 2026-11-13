#!/usr/bin/env python3
"""Optional Linux-only hub memory/load benchmark; no third-party Python packages."""

import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import platform
import resource
import secrets
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid

MIB = 1024 * 1024
ROOT = Path(__file__).resolve().parents[1]


class BenchmarkFailure(Exception):
    """A bounded benchmark requirement was not met."""


def private_file(path, text):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        stream.write(text)


def configuration(path, fields):
    private_file(path, "\n".join(f"{key} = {json.dumps(value, ensure_ascii=False)}" for key, value in fields.items()) + "\n")


def memory(pid):
    result = {}
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            key, _, value = line.partition(":")
            if key in ("VmRSS", "VmHWM", "VmSize"):
                result[key] = int(value.split()[0]) * 1024
    except (FileNotFoundError, ProcessLookupError):
        return None
    return result if "VmRSS" in result else None


class Sampler:
    """Accumulate bounded statistics instead of retaining every sample."""

    def __init__(self, pid, interval):
        self.pid = pid
        self.interval = interval
        self.stop_event = threading.Event()
        self.thread = threading.Thread(target=self.run, name="hub-memory-sampler", daemon=True)
        self.count = 0
        self.peak_rss = 0
        self.peak_hwm = 0
        self.peak_virtual = 0
        self.thread.start()

    def run(self):
        while not self.stop_event.is_set():
            reading = memory(self.pid)
            if reading:
                self.count += 1
                self.peak_rss = max(self.peak_rss, reading["VmRSS"])
                self.peak_hwm = max(self.peak_hwm, reading.get("VmHWM", 0))
                self.peak_virtual = max(self.peak_virtual, reading.get("VmSize", 0))
            self.stop_event.wait(self.interval)

    def finish(self):
        self.stop_event.set()
        self.thread.join(timeout=2)
        reading = memory(self.pid)
        if reading:
            self.count += 1
            self.peak_rss = max(self.peak_rss, reading["VmRSS"])
            self.peak_hwm = max(self.peak_hwm, reading.get("VmHWM", 0))
            self.peak_virtual = max(self.peak_virtual, reading.get("VmSize", 0))
        return {"samples": self.count, "peak_sampled_rss_bytes": self.peak_rss,
                "peak_kernel_hwm_bytes": self.peak_hwm, "peak_virtual_bytes": self.peak_virtual}


def stop_child(child):
    if child is None or child.poll() is not None:
        return
    child.terminate()
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        child.kill()
        child.wait(timeout=5)


def process_alive(child, label):
    if child.poll() is not None:
        raise BenchmarkFailure(f"{label} exited with status {child.returncode}; child logs are suppressed to keep output private")


def wait_until(check, seconds, label, children):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        for name, child in children:
            process_alive(child, name)
        try:
            result = check()
            if result:
                return result
        except (urllib.error.URLError, TimeoutError, ConnectionError):
            pass
        time.sleep(0.1)
    raise BenchmarkFailure(f"timed out waiting for {label} after {seconds:.1f} seconds")


def host_memory_bytes():
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemTotal:"):
            return int(line.split()[1]) * 1024
    return None


def binary_hash(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def rust_version():
    try:
        return subprocess.run(["rustc", "--version"], check=True, capture_output=True,
                              text=True, timeout=10, cwd=ROOT).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return "unavailable; see rust-toolchain.toml"


def database_bytes(directory):
    return {path.name: path.stat().st_size for path in sorted(directory.glob("*.db*")) if path.is_file()}


def benchmark(args):
    if sys.platform != "linux" or not Path("/proc/self/status").exists():
        raise BenchmarkFailure("this benchmark requires Linux /proc and resource limits")
    binaries = {name: args.binary_dir.resolve() / f"zaturo-{name}" for name in ("hub", "worker")}
    for name, path in binaries.items():
        if not path.is_file() or not os.access(path, os.X_OK):
            raise BenchmarkFailure(f"missing executable {path}; run cargo build --release -p zaturo-{name} first")
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    base = f"http://127.0.0.1:{port}"
    owner_token, worker_token = secrets.token_hex(32), secrets.token_hex(32)
    environment = os.environ.copy()
    environment.update(ZATURO_OWNER_TOKEN=owner_token, ZATURO_WORKER_TOKEN=worker_token,
                       RUST_LOG="warn", MALLOC_ARENA_MAX="2")
    worker_environment = environment.copy()
    worker_environment.pop("ZATURO_OWNER_TOKEN", None)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(path, payload=None):
        data = None if payload is None else json.dumps(payload).encode()
        request = urllib.request.Request(base + path, data=data,
                                         headers={"Authorization": f"Bearer {owner_token}", "Content-Type": "application/json"})
        try:
            with opener.open(request, timeout=5) as response:
                return json.load(response)
        except urllib.error.HTTPError as error:
            raise BenchmarkFailure(f"local API request {path} returned HTTP {error.code}") from None

    def address_limit():
        limit = args.hub_address_limit_mib * MIB
        resource.setrlimit(resource.RLIMIT_AS, (limit, limit))

    hub = worker = sampler = None
    report = {"status": "passed", "environment": {
        "kernel": platform.release(), "architecture": platform.machine(), "os": platform.platform(),
        "logical_cpus": os.cpu_count(), "host_memory_bytes": host_memory_bytes(), "rustc": rust_version(),
        "binary_sha256": {name: binary_hash(path) for name, path in binaries.items()}},
        "workload": {"diagnostic_tasks": args.tasks, "http_submission_concurrency": args.concurrency,
                     "worker_task_concurrency": 1, "sampling_interval_ms": args.sample_ms,
                     "idle_seconds": 1, "loopback_only": True},
        "constraints": {"hub_rlimit_as_bytes": args.hub_address_limit_mib * MIB,
                        "malloc_arena_max": 2, "measurement": "hub process /proc VmRSS and VmHWM; worker excluded",
                        "limit_semantics": "RLIMIT_AS caps virtual address space, not RSS; no cgroup memory limit is applied"}}
    with tempfile.TemporaryDirectory(prefix="zaturo-benchmark-") as temporary:
        directory = Path(temporary)
        hub_config, worker_config = directory / "hub.toml", directory / "worker.toml"
        configuration(hub_config, {"bind": f"127.0.0.1:{port}", "client_url": base,
            "database": str(directory / "hub.db"), "worker_id": "benchmark-worker", "max_connections": 16,
            "max_queued_tasks": max(256, args.tasks), "max_page_size": 100, "lease_seconds": 30, "max_workers": 1})
        configuration(worker_config, {"hub_url": base, "worker_id": "benchmark-worker", "database": str(directory / "worker.db"),
            "workspace_root": str(directory / "workspaces"), "artifact_root": str(directory / "artifacts"),
            "capabilities": ["diagnostic.v1"], "max_concurrent_tasks": 1, "heartbeat_seconds": 1, "reconnect_max_seconds": 2})

        def launch_hub():
            # No sampler or submission threads exist when this bounded pre-exec hook runs.
            return subprocess.Popen([str(binaries["hub"]), "--config", str(hub_config), "serve"], cwd=ROOT,
                                    env=environment, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                    stderr=subprocess.DEVNULL, start_new_session=True, preexec_fn=address_limit)

        def connected(since=0):
            return any(item["id"] == "benchmark-worker" and item["last_seen_at"] >= since
                       for item in request("/v1/workers?limit=10"))

        def all_tasks():
            output = []
            for offset in range(0, args.tasks, 100):
                output.extend(request(f"/v1/tasks?limit=100&offset={offset}"))
            return output

        def verified_completed(tasks, expected_ids):
            if {task["id"] for task in tasks} != expected_ids:
                return False
            for task in tasks:
                if task["state"] in ("FAILED", "CANCELLED"):
                    raise BenchmarkFailure("a diagnostic task reached a failed or cancelled terminal state")
                if task["state"] != "COMPLETED":
                    return False
                result = task.get("result") or {}
                if not (result.get("success") is True and result.get("tool") == "diagnostic.v1"
                        and result.get("operation_id") == task["operation_id"] and result.get("os") == "linux"):
                    raise BenchmarkFailure("completed task did not satisfy diagnostic acceptance criteria")
            return True

        try:
            started = time.monotonic()
            hub = launch_hub()
            sampler = Sampler(hub.pid, args.sample_ms / 1000)
            wait_until(lambda: request("/healthz").get("status") == "ok", 15, "hub readiness", [("hub", hub)])
            worker = subprocess.Popen([str(binaries["worker"]), "--config", str(worker_config), "connect"], cwd=ROOT,
                                      env=worker_environment, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                      stderr=subprocess.DEVNULL, start_new_session=True)
            wait_until(connected, 15, "authenticated worker registration", [("hub", hub), ("worker", worker)])
            time.sleep(1)
            idle = memory(hub.pid)
            if not idle:
                raise BenchmarkFailure("could not sample idle hub memory")
            report["idle"] = {"rss_bytes": idle["VmRSS"], "virtual_bytes": idle["VmSize"]}
            load_started = time.monotonic()

            def submit(index):
                task = request("/v1/tasks", {"kind": "diagnostic", "request": "Run the approved benchmark diagnostic.",
                    "target_worker": "benchmark-worker", "idempotency_key": f"benchmark-{index}-{uuid.uuid4().hex}",
                    "priority": 0, "max_attempts": 3, "timeout_seconds": 300})
                return task["id"]

            with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as executor:
                expected_ids = set(executor.map(submit, range(args.tasks)))
            if len(expected_ids) != args.tasks:
                raise BenchmarkFailure("task submission returned duplicate identifiers")
            report["submission_seconds"] = round(time.monotonic() - load_started, 6)
            completed = wait_until(lambda: verified_completed(all_tasks(), expected_ids), args.tasks * 3 + 20,
                                   "all diagnostic completions", [("hub", hub), ("worker", worker)])
            report["load_seconds"] = round(time.monotonic() - load_started, 6)
            report["completed_tasks"] = len(expected_ids) if completed else 0
            report["hub_memory"] = sampler.finish()
            sampler = None
            if not args.no_restart_check:
                stop_child(hub)
                restarted_at = int(time.time() * 1000)
                hub = launch_hub()
                sampler = Sampler(hub.pid, args.sample_ms / 1000)
                wait_until(lambda: request("/healthz").get("status") == "ok", 15, "restarted hub readiness", [("hub", hub), ("worker", worker)])
                wait_until(lambda: connected(restarted_at), 15, "worker reconnection", [("hub", hub), ("worker", worker)])
                if not verified_completed(all_tasks(), expected_ids):
                    raise BenchmarkFailure("completed task metadata was lost after hub restart")
                sample_id = sorted(expected_ids)[0]
                events = request(f"/v1/tasks/{sample_id}/events?limit=100")
                if len(events) < 4 or events[-1]["state"] != "COMPLETED":
                    raise BenchmarkFailure("completed task history was not preserved after hub restart")
                report["restart"] = {"completed_metadata_preserved": True, "history_events_verified": len(events),
                                     "worker_reconnected": True, "hub_memory": sampler.finish()}
                sampler = None
            else:
                report["restart"] = {"status": "skipped by argument"}
            report["database_files_bytes"] = database_bytes(directory)
            report["total_seconds"] = round(time.monotonic() - started, 6)
            overall_hwm = max(report["hub_memory"]["peak_kernel_hwm_bytes"],
                              report["restart"].get("hub_memory", {}).get("peak_kernel_hwm_bytes", 0))
            report["target_observations"] = {
                "idle_rss_below_150_mib": report["idle"]["rss_bytes"] < 150 * MIB,
                "peak_hwm_below_300_mib": overall_hwm < 300 * MIB,
                "scope": "this synthetic workload only; these observations are not a production capacity guarantee"}
            return report
        finally:
            if sampler is not None:
                sampler.finish()
            stop_child(worker)
            stop_child(hub)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary-dir", type=Path, default=ROOT / "target/release")
    parser.add_argument("--tasks", type=int, default=30, help="diagnostic tasks, from 1 through 200")
    parser.add_argument("--concurrency", type=int, default=4, help="HTTP submission concurrency, from 1 through 8")
    parser.add_argument("--sample-ms", type=int, default=50, help="memory sampling interval, from 10 through 1000 milliseconds")
    parser.add_argument("--hub-address-limit-mib", type=int, default=300)
    parser.add_argument("--no-restart-check", action="store_true")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if not 1 <= args.tasks <= 200 or not 1 <= args.concurrency <= 8 or not 10 <= args.sample_ms <= 1000:
        parser.error("tasks must be 1..200, concurrency 1..8, and sample-ms 10..1000")
    if not 128 <= args.hub_address_limit_mib <= 2048:
        parser.error("hub-address-limit-mib must be 128..2048")
    try:
        report = benchmark(args)
    except (BenchmarkFailure, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        report = {"status": "failed", "error": str(error)}
    text = json.dumps(report, indent=2, sort_keys=True)
    print(text)
    if args.output:
        args.output.write_text(text + "\n", encoding="utf-8")
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
