#!/usr/bin/env python3
"""Bounded model-independent Linux MCP bridge. An application allowlist is not an OS sandbox.

Wire/schema reference: agent-sh/computer-use-linux v0.7.53, commit below, src/server.rs.
The private installer must independently verify the release artifact before recording its hash.
No upstream installer, desktop setup, shell, HTTP listener or arbitrary argv is exposed here.
"""
from __future__ import annotations

import argparse
import base64
import contextlib
import ctypes
import fcntl
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import selectors
import select
import signal
import stat
import struct
import subprocess
import sys
import time
import tomllib
import uuid
import zlib

UPSTREAM_VERSION = "0.7.53"
UPSTREAM_COMMIT = "4ee79ac80bb4db157892cf68f0d6fba5547538be"
UPSTREAM_URL = "https://github.com/agent-sh/computer-use-linux/tree/" + UPSTREAM_COMMIT
PROTOCOL = "2025-11-25"
MAX_JSON = 32768
MAX_MCP = 4 * 1024 * 1024
MAX_TEXT = 8192
MAX_IMAGE = 2 * 1024 * 1024
MAX_PRIVATE_IMAGE = 8 * 1024 * 1024
DENIED = re.compile(r"terminal|xterm|konsole|alacritty|kitty|wezterm|tilix|foot|powershell|keyring|keepass|bitwarden|1password|password|passwd|credential|polkit|systemsettings|gnome-control-center|gnome-shell|dconf|gparted|krunner|rofi|dmenu|ulauncher|appfinder|gmrun|seahorse|kwallet|gksu|policykit", re.I)
PROTECTED = re.compile(r"password|senha|passphrase|credential|api[_ -]?key|access[_ -]?token|one.time|verification.code|authentication.code|2fa|código.de", re.I)
BROWSER_APP = re.compile(r"firefox|chrome|chromium|brave|vivaldi|microsoft.?edge|opera|librewolf|zen.browser|epiphany|falkon|qutebrowser", re.I)
SEMANTIC_RUNNER = re.compile(r"codex|zaturo|opencode|claude|cursor|vscodium|code[-._]?oss|visual.?studio.?code|com\.visualstudio\.code|jetbrains|intellij|pycharm|webstorm|android.?studio|eclipse|sublime|geany|zed(?:$|[-._])", re.I)
SECRET = re.compile(r"oc_sk_|AGE-SECRET-|-----BEGIN |\bsk-[A-Za-z0-9]|access_token\s*[=:]|api[_ -]?key\s*[=:]", re.I)
INPUT_TOOLS = {"click", "perform_action", "set_value", "type_text", "press_key", "scroll", "activate_window"}
READ_TOOLS = {"list_windows", "get_app_state", "get_tree_page"}
KEYS = {"Tab", "Shift+Tab", "Escape", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End", "PageUp", "PageDown", "Backspace", "Delete", "Enter"}
ENV_NAMES = {"HOME", "USER", "LOGNAME", "LANG", "DISPLAY", "WAYLAND_DISPLAY", "XAUTHORITY", "XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS", "XDG_SESSION_TYPE", "XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP", "DESKTOP_SESSION", "HYPRLAND_INSTANCE_SIGNATURE", "NIRI_SOCKET", "SWAYSOCK"}


class Rejected(Exception):
    def __init__(self, code):
        self.code = code
        self.dispatched = False
        super().__init__(code)


def need(condition, code="invalid_request"):
    if not condition:
        raise Rejected(code)


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


def digest(value):
    return hashlib.sha256(encoded(value)).hexdigest()


def plan_digest(request):
    """Digest exactly what the owner approved; lease secrets are deliberately excluded."""
    return digest({key: request.get(key) for key in ("tool", "arguments", "target", "expected_snapshot_sha256")})


def private_read(path, limit=MAX_JSON):
    path = Path(path)
    need(path.is_absolute(), "private_path_required")
    need(not any(p.is_symlink() for p in [path, *path.parents]), "unsafe_private_path")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        need(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & 0o077 and info.st_size <= limit, "private_file_required")
        data = stream.read(limit + 1)
        need(len(data) <= limit, "response_too_large")
        return data


def safe_regular(path, maximum=256 * 1024 * 1024):
    path = Path(path)
    need(path.is_absolute() and not any(p.is_symlink() for p in [path, *path.parents]), "unsafe_executable")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        need(stat.S_ISREG(info.st_mode) and info.st_uid in (0, os.getuid()) and not info.st_mode & 0o022 and info.st_size <= maximum, "unsafe_executable")
        result = hashlib.file_digest(stream, "sha256").hexdigest()
    return path, result


def checked_artifact(path, expected):
    need(isinstance(expected, str) and re.fullmatch(r"[0-9a-f]{64}", expected), "artifact_hash_required")
    artifact, actual = safe_regular(path)
    need(actual == expected, "artifact_hash_mismatch")
    return artifact


def clean_text(value, maximum=256):
    if not isinstance(value, str):
        return ""
    if SECRET.search(value):
        return "[redacted]"
    return "".join(c for c in value if c in "\n\t" or ord(c) >= 32).encode()[:maximum].decode("utf-8", errors="ignore")


def denied_text(value):
    return bool(DENIED.search(str(value)))


class Config:
    def __init__(self, path, *, backend="desktop"):
        self.path = Path(path)
        self.backend = backend
        self.value = tomllib.loads(private_read(self.path).decode())
        common = {"enabled", "state_dir", "timeout_seconds", "allow_input", "allow_capture"}
        fields = {"binary", "binary_sha256", "allowed_apps"} if backend == "desktop" else {"node", "node_sha256", "playwright_cli", "playwright_cli_sha256", "playwright_tree_sha256", "cdp_endpoint", "profile_dir", "browser_pid", "allowed_origins"}
        need(set(self.value) <= common | fields, "invalid_configuration")
        need(self.value.get("enabled", False) is True, "backend_disabled")
        if backend == "browser":
            tree_hash = self.value.get("playwright_tree_sha256")
            need(isinstance(tree_hash, str) and re.fullmatch(r"[0-9a-f]{64}", tree_hash), "package_tree_hash_required")
        self.allow_input = self.value.get("allow_input", False) is True
        self.allow_capture = self.value.get("allow_capture", False) is True
        self.timeout = self.value.get("timeout_seconds", 15)
        need(type(self.timeout) is int and 1 <= self.timeout <= 30, "invalid_configuration")
        self.state = Path(self.value["state_dir"])
        need(self.state.is_absolute() and not any(p.is_symlink() for p in [self.state, *self.state.parents]), "unsafe_private_path")
        self.state.mkdir(mode=0o700, parents=True, exist_ok=True)
        info = self.state.stat()
        need(info.st_uid == os.getuid() and stat.S_ISDIR(info.st_mode) and not info.st_mode & 0o077, "private_directory_required")
        self.stop = self.state / "STOP"
        if backend == "desktop":
            self.binary = checked_artifact(self.value["binary"], self.value["binary_sha256"])
            need(self.binary.name == "computer-use-linux" and os.access(self.binary, os.X_OK), "invalid_executable")
            self.apps = self.value.get("allowed_apps", [])
            need(isinstance(self.apps, list) and 1 <= len(self.apps) <= 16 and all(isinstance(v, str) and 1 <= len(v) <= 128 and not denied_text(v) for v in self.apps), "invalid_app_grant")

    def check(self):
        need(not self.stop.exists() and not self.stop.is_symlink(), "emergency_stop")


def context(request):
    fields = {"operation_id", "lease_token", "lease_expires_at", "deadline_at", "owner_confirmed", "target", "tool", "arguments", "expected_snapshot_sha256", "approved_plan_sha256"}
    need(isinstance(request, dict) and set(request) <= fields, "invalid_request")
    try:
        need(str(uuid.UUID(request["operation_id"])) == request["operation_id"])
    except (ValueError, KeyError, TypeError, AttributeError):
        raise Rejected("invalid_operation") from None
    token = request.get("lease_token")
    need(isinstance(token, str) and 16 <= len(token) <= 256 and all(33 <= ord(c) <= 126 for c in token), "invalid_lease")
    now = int(time.time() * 1000)
    lease, end = request.get("lease_expires_at"), request.get("deadline_at")
    need(type(lease) is int and type(end) is int and now < lease <= now + 300000 and now < end <= now + 300000, "expired_lease")
    need(type(request.get("owner_confirmed", False)) is bool and isinstance(request.get("arguments"), dict), "invalid_request")
    return min(lease, end)


def input_grant(config, request):
    need(config.allow_input and request.get("owner_confirmed") is True, "input_grant_required")
    need(request.get("approved_plan_sha256") == plan_digest(request), "exact_plan_required")
    need(isinstance(request.get("expected_snapshot_sha256"), str) and re.fullmatch(r"[0-9a-f]{64}", request["expected_snapshot_sha256"]), "snapshot_required")


@contextlib.contextmanager
def operation(config, request, mutating=False):
    """Serialize one local action; a started uncertain mutation is never dispatched again."""
    end = context(request)
    config.check()
    lock = config.state / "bridge.lock"
    fd = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    try:
        need(not os.fstat(fd).st_mode & 0o077, "private_file_required")
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise Rejected("desktop_busy") from None
        intent = config.state / (request["operation_id"] + ".json")
        fingerprint = digest({"plan": plan_digest(request), "lease": hashlib.sha256(request["lease_token"].encode()).hexdigest()})
        dispatched = False
        def begin():
            nonlocal dispatched
            config.check()
            need(int(time.time() * 1000) < end, "expired_lease")
            if mutating:
                if intent.exists() or intent.is_symlink():
                    previous = json.loads(private_read(intent))
                    need(previous.get("fingerprint") == fingerprint, "operation_conflict")
                    raise Rejected("reconciliation_required")
                need(sum(1 for _ in itertools.islice(config.state.glob("*.json"), 10001)) < 10000, "journal_capacity")
                descriptor = os.open(intent, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
                with os.fdopen(descriptor, "wb") as out:
                    out.write(encoded({"fingerprint": fingerprint, "state": "dispatched"})); out.flush(); os.fsync(out.fileno())
                dispatched = True
        try:
            yield end, begin
        except Rejected as error:
            error.dispatched = dispatched
            raise
        except BaseException:
            if dispatched:
                error = Rejected("input_uncertain"); error.dispatched = True
                raise error from None
            raise
    finally:
        os.close(fd)


class Session:
    """One bounded newline JSON-RPC session; stderr is drained but never returned/logged."""
    def __init__(self, config, argv, end=None, expected_version=UPSTREAM_VERSION):
        self.config = config
        self.deadline = min(time.monotonic() + config.timeout, time.monotonic() + max(0, ((end or int(time.time()*1000)+30000) - time.time()*1000)/1000))
        env = {k: v for k, v in os.environ.items() if k in ENV_NAMES or k.startswith("LC_")}
        env.update(PATH="/usr/bin:/bin", LANG="C.UTF-8", COMPUTER_USE_LINUX_INDICATOR_HIDE_TEXT="1")
        if config.backend == "desktop":
            # Pinned upstream GNOME/portal/CLI capture routes use temporary PNG
            # files. Native X11 GetImage is in memory and this override has no
            # fallback; Wayland private capture stays unavailable.
            env["COMPUTER_USE_LINUX_SCREENSHOT_BACKEND"] = "x11"
        # Shell, completion notifications and persisted portal grants never inherit ambient values.
        env["COMPUTER_USE_LINUX_INDICATOR"] = "0"
        config.check()
        # The guardian survives an outer-helper SIGKILL, sees pipe EOF and reaps its own
        # process tree. Backend commands may create process groups of their own.
        self.process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()), "--internal-guard", str(config.path), config.backend], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, cwd=config.state, start_new_session=True, bufsize=0)
        self.reader = selectors.DefaultSelector()
        self.reader.register(self.process.stdout, selectors.EVENT_READ, "stdout")
        self.reader.register(self.process.stderr, selectors.EVENT_READ, "stderr")
        self.buffer = bytearray(); self.total = 0; self.sequence = 0
        try:
            reply = self.rpc("initialize", {"protocolVersion": PROTOCOL, "capabilities": {}, "clientInfo": {"name": "zaturobot-bounded-linux", "version": "0.6.0"}})
            need(reply.get("protocolVersion") in {PROTOCOL, "2025-06-18", "2025-03-26", "2024-11-05"}, "unsupported_mcp")
            need(reply.get("serverInfo", {}).get("version") == expected_version, "upstream_version_mismatch")
            self.write({"jsonrpc": "2.0", "method": "notifications/initialized"})
        except BaseException:
            self.close(); raise

    def check(self):
        self.config.check()
        need(time.monotonic() < self.deadline, "mcp_timeout")

    def write(self, value):
        data = encoded(value) + b"\n"
        need(len(data) <= MAX_JSON, "request_too_large")
        self.check()
        try:
            self.process.stdin.write(data); self.process.stdin.flush()
        except (BrokenPipeError, OSError):
            raise Rejected("mcp_unavailable") from None

    def line(self):
        while True:
            self.check()
            index = self.buffer.find(b"\n")
            if index >= 0:
                raw = self.buffer[:index]; del self.buffer[:index+1]
                need(len(raw) <= MAX_MCP, "response_too_large")
                return raw
            need(len(self.buffer) <= MAX_MCP, "response_too_large")
            events = self.reader.select(min(.05, max(0, self.deadline-time.monotonic())))
            for key, _ in events:
                chunk = os.read(key.fileobj.fileno(), 65536)
                if not chunk:
                    self.reader.unregister(key.fileobj)
                    if key.data == "stdout":
                        raise Rejected("mcp_unavailable")
                    continue
                self.total += len(chunk)
                need(self.total <= MAX_MCP * 4, "response_too_large")
                if key.data == "stdout":
                    self.buffer.extend(chunk)

    def rpc(self, method, params):
        self.sequence += 1; identity = self.sequence
        self.write({"jsonrpc": "2.0", "id": identity, "method": method, "params": params})
        for _ in range(64):
            try:
                message = json.loads(self.line())
            except (ValueError, UnicodeError):
                raise Rejected("invalid_mcp") from None
            need(isinstance(message, dict) and message.get("jsonrpc") == "2.0", "invalid_mcp")
            if "method" in message:
                if "id" in message:
                    self.write({"jsonrpc": "2.0", "id": message["id"], "error": {"code": -32601, "message": "unsupported"}})
                continue
            need(message.get("id") == identity and "error" not in message, "mcp_rejected")
            need(isinstance(message.get("result"), dict), "invalid_mcp")
            return message["result"]
        raise Rejected("response_too_large")

    def tool(self, name, arguments, *, image=False):
        result = self.rpc("tools/call", {"name": name, "arguments": arguments})
        need(result.get("isError") is not True, "tool_rejected")
        blocks = result.get("content", [])
        need(isinstance(blocks, list) and len(blocks) <= 16, "invalid_mcp")
        if image:
            return result
        need(not any(b.get("type") == "image" for b in blocks if isinstance(b, dict)), "unexpected_image")
        value = result.get("structuredContent")
        if value is None:
            texts = [b.get("text", "") for b in blocks if isinstance(b, dict) and b.get("type") == "text"]
            need(len(texts) == 1, "invalid_mcp")
            try:
                value = json.loads(texts[0])
            except (ValueError, TypeError):
                raise Rejected("invalid_mcp") from None
        need(isinstance(value, dict), "invalid_mcp")
        return value

    def close(self):
        if hasattr(self, "process"):
            try:
                self.process.stdin.close()
            except (BrokenPipeError, OSError):
                pass
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                try: os.killpg(self.process.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                self.process.wait(timeout=1)
            for stream in [self.process.stdin, self.process.stdout, self.process.stderr]:
                stream.close()
            self.reader.close()
            self.buffer[:] = b"\0" * len(self.buffer)

    def __enter__(self): return self
    def __exit__(self, *_): self.close()


def _own_descendants():
    own = os.getpid(); discovered = {own}; pending = [own]
    while pending and len(discovered) <= 1024:
        parent = pending.pop()
        try: children = Path(f"/proc/{parent}/task/{parent}/children").read_text().split()
        except OSError: continue
        for raw in children:
            if raw.isdecimal() and int(raw) not in discovered:
                pid = int(raw); discovered.add(pid); pending.append(pid)
    return discovered


def _reap_descendants():
    for _ in range(6):
        owned = _own_descendants()
        for pid in sorted(owned - {os.getpid()}, reverse=True):
            descriptor = None
            try:
                # A pidfd plus a repeated parent check avoids signaling a recycled PID.
                descriptor = os.pidfd_open(pid)
                status = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
                if int(status[1]) in owned:
                    signal.pidfd_send_signal(descriptor, signal.SIGKILL)
            except (OSError, ValueError, ProcessLookupError): pass
            finally:
                if descriptor is not None: os.close(descriptor)
        while True:
            try:
                child, _ = os.waitpid(-1, os.WNOHANG)
                if child == 0: break
            except ChildProcessError: break
        if len(_own_descendants()) == 1: return
        time.sleep(.03)


def guardian(config_path, backend):
    """Internal fixed backend proxy. EOF and watchdog clean even escaped grandchildren.

    This Linux subreaper is process ownership, not filesystem/network isolation.
    """
    child = None
    try:
        need(backend in {"desktop", "browser"}, "invalid_configuration")
        config = Config(Path(config_path), backend=backend)
        if backend == "desktop": argv = [str(config.binary), "mcp"]
        else:
            import browser_mcp
            argv = browser_mcp.backend_argv(config)
        libc = ctypes.CDLL(None, use_errno=True)
        need(libc.prctl(36, 1, 0, 0, 0) == 0 and hasattr(os, "pidfd_open") and hasattr(signal, "pidfd_send_signal"), "supervision_unavailable")
        child = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, close_fds=True, bufsize=0)
        destinations = {0: child.stdin.fileno(), child.stdout.fileno(): 1, child.stderr.fileno(): 2}
        buffers = {fd: bytearray() for fd in destinations.values()}
        for fd in {*destinations, *destinations.values()}: os.set_blocking(fd, False)
        deadline = time.monotonic() + config.timeout + 2
        while time.monotonic() < deadline:
            config.check()
            reads, writes, _ = select.select(list(destinations), [fd for fd, value in buffers.items() if value], [], .03)
            for fd in reads:
                try: data = os.read(fd, 65536)
                except BlockingIOError: continue
                if not data: return 0
                pending = buffers[destinations[fd]]; pending.extend(data)
                need(len(pending) <= MAX_MCP * 4, "response_too_large")
            for fd in writes:
                try: count = os.write(fd, buffers[fd][:65536])
                except BlockingIOError: continue
                del buffers[fd][:count]
        return 1
    except BaseException:
        return 1
    finally:
        _reap_descendants()
        if child is not None:
            for stream in [child.stdin, child.stdout, child.stderr]: stream.close()


def target_valid(config, target):
    need(isinstance(target, dict) and set(target) == {"window_id", "pid", "app_id"}, "invalid_target")
    need(type(target["window_id"]) is int and 0 < target["window_id"] <= 2**53-1 and type(target["pid"]) is int and 1 < target["pid"] <= 2**32-1, "invalid_target")
    need(target["app_id"] in config.apps and not denied_text(target["app_id"]), "app_not_granted")


def window_check(session, target, *, focused=False, semantic=False):
    windows = session.tool("list_windows", {}).get("windows")
    need(isinstance(windows, list) and len(windows) <= 256, "invalid_mcp")
    matched = [w for w in windows if isinstance(w, dict) and w.get("window_id") == target["window_id"]]
    need(len(matched) == 1, "target_unavailable")
    value = matched[0]
    need(value.get("pid") == target["pid"] and value.get("app_id") == target["app_id"], "target_changed")
    need(not value.get("terminal") and not denied_text(" ".join(str(value.get(k, "")) for k in ("app_id", "wm_class", "title"))), "protected_application")
    if semantic:
        need(not BROWSER_APP.search(" ".join(str(value.get(k, "")) for k in ("app_id", "wm_class"))), "use_dedicated_browser_backend")
        need(not SEMANTIC_RUNNER.search(" ".join(str(value.get(k, "")) for k in ("app_id", "wm_class"))), "protected_agent_or_runner")
    need(not value.get("hidden", False), "target_unavailable")
    if focused: need(value.get("focused") is True, "owner_takeover")
    return {**target, "title": clean_text(value.get("title"), 160), "focused": value.get("focused") is True}


def compact_state(session, target):
    data = session.tool("get_app_state", {**target, "include_screenshot": False, "max_nodes": 64, "max_depth": 12, "interactive_only": True, "verbose": False})
    need(not data.get("accessibility_error") and not data.get("window_error") and data.get("tree_scoped") is True, "accessibility_unavailable")
    window = data.get("window_context", {})
    need(isinstance(window, dict) and all(window.get(k) == v for k, v in target.items()), "target_changed")
    snapshot = data.get("snapshot_id")
    need(isinstance(snapshot, str) and 1 <= len(snapshot) <= 160, "snapshot_required")
    tree = data.get("accessibility_tree")
    need(isinstance(tree, list) and len(tree) <= 64, "invalid_mcp")
    nodes = []
    for node in tree:
        need(isinstance(node, dict) and type(node.get("index")) is int, "invalid_mcp")
        protected = denied_text(node.get("role", "")) or PROTECTED.search(str(node.get("name", ""))) is not None or "protected" in node.get("states", [])
        text = node.get("text", {})
        text = text.get("content", "") if isinstance(text, dict) else ""
        nodes.append({"index": node["index"], "role": clean_text(node.get("role"), 64), "name": "[protected]" if protected else clean_text(node.get("name"), 160), "text": "" if protected else clean_text(text, 160), "actions": [clean_text(a, 40) for a in node.get("actions", [])[:8] if isinstance(a, str)], "states": [clean_text(s, 32) for s in node.get("states", [])[:12] if isinstance(s, str)], "editable": node.get("supports_editable_text") is True, "protected": protected})
    result = {"target": target, "nodes": nodes, "truncated": data.get("accessibility_tree_truncated") is True}
    need(len(encoded(result)) <= 65536, "semantic_output_too_large")
    return result, snapshot, digest(result)


def state_page(state, offset=0):
    count = len(state["nodes"])
    view = {**state, "nodes": state["nodes"][offset:offset+16], "total_nodes": count, "offset": offset}
    while view["nodes"] and len(encoded(view)) > MAX_TEXT - 96:
        view["nodes"].pop()
    view["next_offset"] = offset+len(view["nodes"]) if offset+len(view["nodes"]) < count else None
    view["truncated"] = state["truncated"] or view["next_offset"] is not None
    need(len(encoded(view)) <= MAX_TEXT, "semantic_output_too_large")
    return view


def action_arguments(request, state, upstream_snapshot):
    tool, args = request["tool"], dict(request["arguments"])
    fields = {
        "click": {"element_index", "button", "click_count"},
        "perform_action": {"element_index", "action"},
        "set_value": {"element_index", "value"},
        "type_text": {"element_index", "text"},
        "press_key": {"element_index", "key"},
        "scroll": {"element_index", "direction", "pages"},
        "activate_window": set(),
    }
    need(tool in fields and not set(args) - fields[tool], "tool_not_allowed")
    if tool != "activate_window":
        need(type(args.get("element_index")) is int, "element_required")
        matches = [n for n in state["nodes"] if n["index"] == args["element_index"]]
        need(len(matches) == 1 and not matches[0]["protected"], "protected_element")
        node = matches[0]
        if tool in {"type_text", "press_key"}:
            need("focused" in node["states"], "focus_changed")
        if tool in {"set_value", "type_text"}:
            need(node["editable"] and not denied_text(node["role"]), "editable_element_required")
            text = args.get("value" if tool == "set_value" else "text")
            need(isinstance(text, str) and 0 < len(text.encode()) <= 1024 and not any(ord(c) < 32 for c in text) and not SECRET.search(text), "invalid_input_text")
            need(not re.search(r"javascript:|file:|data:|chrome://|about:|\$\(|`", text, re.I), "unsafe_input_text")
        if tool == "press_key": need(args.get("key") in KEYS, "key_not_allowed")
        if tool == "click":
            need(args.get("button", "left") in {"left", "right"} and type(args.get("click_count", 1)) is int and args.get("click_count", 1) in {1, 2}, "invalid_click")
        if tool == "perform_action":
            need(args.get("action") in node["actions"], "action_not_observed")
        if tool == "scroll":
            need(args.get("direction") in {"up", "down", "left", "right"} and type(args.get("pages", 1)) is int and 1 <= args.get("pages", 1) <= 3, "invalid_scroll")
        if tool not in {"type_text", "press_key"}:
            args["snapshot_id"] = upstream_snapshot
        else:
            del args["element_index"]
    args.update(request["target"])
    args.update(screenshot_after=False, include_tree=False)
    return args


def execute(config, request):
    tool = request.get("tool")
    need(tool in READ_TOOLS | INPUT_TOOLS, "tool_not_allowed")
    target_valid(config, request.get("target"))
    # Desktop input cannot evade the dedicated browser profile/origin policy. A separately
    # confirmed private capture may still deliver browser pixels only to the owner.
    need(not BROWSER_APP.search(request["target"]["app_id"]), "use_dedicated_browser_backend")
    need(not SEMANTIC_RUNNER.search(request["target"]["app_id"]), "protected_agent_or_runner")
    mutating = tool in INPUT_TOOLS
    if mutating: input_grant(config, request)
    elif tool == "get_tree_page":
        need(set(request["arguments"]) <= {"offset"} and type(request["arguments"].get("offset", 0)) is int and 0 <= request["arguments"].get("offset", 0) < 64, "invalid_arguments")
    else: need(not request["arguments"], "invalid_arguments")
    with operation(config, request, mutating) as (end, begin):
        with Session(config, [config.binary, "mcp"], end) as session:
            window = window_check(session, request["target"], semantic=True)
            if tool == "list_windows":
                return {"schema": 1, "status": "completed", "tool": tool, "data": {"windows": [window]}}
            state, snapshot, fingerprint = compact_state(session, request["target"])
            if not mutating:
                if tool == "get_tree_page":
                    need(request.get("expected_snapshot_sha256") == fingerprint, "stale_snapshot")
                offset = request["arguments"].get("offset", 0)
                return {"schema": 1, "status": "completed", "tool": tool, "data": state_page(state, offset), "snapshot_sha256": fingerprint}
            need(request["expected_snapshot_sha256"] == fingerprint, "stale_snapshot")
            arguments = action_arguments(request, state, snapshot)
            if tool != "activate_window": window_check(session, request["target"], focused=True, semantic=True)
            begin()
            response = session.tool(tool, arguments)
            need(response.get("ok") is True, "input_uncertain")
            window_check(session, request["target"], focused=True, semantic=True)
            session.check()
            return {"schema": 1, "status": "completed", "tool": tool, "data": {"dispatched": True, "verified_external_effect": False, "target": request["target"]}}


def doctor(config):
    with Session(config, [config.binary, "mcp"]) as session:
        report = session.tool("doctor", {})
        windows = session.tool("list_windows", {}).get("windows", [])
    readiness = report.get("readiness", {})
    need(isinstance(readiness, dict), "invalid_mcp")
    keys = {"can_register_mcp_tools", "can_build_accessibility_tree", "can_send_development_input", "can_query_windows", "can_capture_screenshots"}
    need(isinstance(windows, list) and len(windows) <= 256, "invalid_mcp")
    visible = []
    for window in windows:
        if isinstance(window, dict) and window.get("app_id") in config.apps and not window.get("terminal") and not denied_text(" ".join(str(window.get(k, "")) for k in ("app_id", "wm_class", "title"))):
            visible.append({k: window.get(k) for k in ("window_id", "pid", "app_id", "focused")})
    return {"schema": 1, "status": "completed", "upstream": UPSTREAM_VERSION, "data": {"readiness": {k: v for k, v in readiness.items() if k in keys and type(v) is bool}, "windows": visible[:8], "capture_verified": False, "input_granted": config.allow_input, "capture_granted": config.allow_capture, "stop_file": str(config.stop)}}


def capture_png(config, request):
    """Private-view caller only: return a bounded PNG bytearray in RAM, never JSON/base64/files.

    The caller must encrypt, expire and wipe it. Python cannot promise total heap erasure.
    No ordinary CLI action exposes this function or returns image content.
    """
    need(config.allow_capture and request.get("owner_confirmed") is True and request.get("tool") == "screenshot" and not request.get("arguments"), "capture_grant_required")
    need(request.get("approved_plan_sha256") == plan_digest(request), "exact_plan_required")
    target_valid(config, request.get("target"))
    with operation(config, request, True) as (end, begin):
        with Session(config, [config.binary, "mcp"], end) as session:
            window_check(session, request["target"], focused=True)
            begin()
            reply = session.tool("get_app_state", {**request["target"], "include_screenshot": True, "max_nodes": 1, "max_depth": 1, "format": "png", "max_width": 1920, "max_height": 1920, "max_bytes": MAX_IMAGE}, image=True)
            structured = reply.get("structuredContent", {})
            need(isinstance(structured, dict) and not structured.get("screenshot_error") and isinstance(structured.get("window_context"), dict) and all(structured["window_context"].get(k) == v for k, v in request["target"].items()), "target_changed")
            images = [b for b in reply.get("content", []) if isinstance(b, dict) and b.get("type") == "image"]
            need(len(images) == 1 and images[0].get("mimeType") == "image/png", "invalid_private_image")
            payload = images[0].get("data")
            need(isinstance(payload, str) and len(payload) <= (MAX_IMAGE+2)//3*4, "invalid_private_image")
            try: image = bytearray(base64.b64decode(payload, validate=True))
            except (ValueError, TypeError): raise Rejected("invalid_private_image") from None
            try:
                validate_png(image)
                window_check(session, request["target"], focused=True); session.check()
                return image
            except BaseException:
                image[:] = b"\0" * len(image); raise


def validate_png(image):
    need(57 <= len(image) <= MAX_PRIVATE_IMAGE and image[:8] == b"\x89PNG\r\n\x1a\n", "invalid_private_image")
    offset = 8; header = False; data = False; ended = False
    while offset + 12 <= len(image):
        length = struct.unpack(">I", image[offset:offset+4])[0]
        need(length <= MAX_PRIVATE_IMAGE and offset + 12 + length <= len(image), "invalid_private_image")
        tag = image[offset+4:offset+8]; payload = image[offset+8:offset+8+length]
        crc = struct.unpack(">I", image[offset+8+length:offset+12+length])[0]
        need(zlib.crc32(tag+payload) & 0xffffffff == crc, "invalid_private_image")
        if not header:
            need(tag == b"IHDR" and length == 13, "invalid_private_image")
            width, height = struct.unpack(">II", payload[:8])
            need(1 <= width <= 4096 and 1 <= height <= 4096 and width*height <= 16*1024*1024, "invalid_private_image")
            header = True
        elif tag == b"IHDR": raise Rejected("invalid_private_image")
        if tag == b"IDAT": data = True
        offset += 12 + length
        if tag == b"IEND":
            need(length == 0 and data and offset == len(image), "invalid_private_image"); ended = True; break
    need(ended, "invalid_private_image")


def result_error(error):
    code = error.code if isinstance(error, Rejected) else "bridge_unavailable"
    uncertain = getattr(error, "dispatched", False) or code in {"input_uncertain", "reconciliation_required", "mcp_timeout", "mcp_unavailable", "owner_takeover"}
    return {"schema": 1, "status": "uncertain" if uncertain else "blocked", "code": code, "summary": "A ação não foi confirmada. Confira o estado antes de repetir." if uncertain else "A ação foi bloqueada. Verifique a permissão local, a janela selecionada e a conexão."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("command", choices=["doctor", "action", "capture"])
    parser.add_argument("--request-json", type=Path)
    args = parser.parse_args(); os.umask(0o077)
    try:
        config = Config(args.config)
        if args.command == "doctor": need(args.request_json is None); result = doctor(config)
        elif args.command == "capture":
            need(args.request_json is not None)
            image = capture_png(config, json.loads(private_read(args.request_json)))
            try: sys.stdout.buffer.write(image); sys.stdout.buffer.flush()
            finally: image[:] = b"\0" * len(image)
            return 0
        else:
            need(args.request_json is not None)
            result = execute(config, json.loads(private_read(args.request_json)))
    except (Rejected, OSError, ValueError, KeyError, TypeError) as error:
        result = result_error(error)
        if args.command == "capture":
            print(json.dumps(result, ensure_ascii=False, separators=(",", ":")), file=sys.stderr)
            return 1
    print(json.dumps(result, ensure_ascii=False, separators=(",", ":")))
    return 0 if result["status"] == "completed" else 1


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--internal-guard":
        raise SystemExit(guardian(sys.argv[2], sys.argv[3]))
    raise SystemExit(main())
