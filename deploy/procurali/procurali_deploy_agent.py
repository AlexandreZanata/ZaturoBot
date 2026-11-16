#!/usr/bin/env python3
"""Pull-based zero-downtime blue-green auto-deploy agent for Procurali on Linux VPS.

Polls GitHub Releases produced only after CI checks succeed on `main`, independently
verifies all required GitHub check-runs (`check` and `android`) and SHA-256 digests,
takes an online SQLite backup, boots the standby Hub slot (`a` <-> `b`), health-checks it,
gracefully reloads Caddy, rebuilds static assets behind an atomic symlink, drains the old
slot, and rolls back automatically if post-cutover verification fails.
"""

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import os
import shutil
import stat
import subprocess
import sys
import tarfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable


SLOTS = {"a": 17881, "b": 17882}
DEFAULT_REPO = "AlexandreZanata/ZaturoBot"
DEFAULT_REQUIRED_CHECKS = ("check", "android")
BUNDLE_ASSET_NAME = "procurali-bundle-linux-amd64.tar.gz"
MANIFEST_ASSET_NAME = "manifest.json"
USER_AGENT = "procurali-deploy-agent/1.0"


def log(event: str, **fields: object) -> None:
    print(json.dumps({"event": event, **fields}, ensure_ascii=False), flush=True)


@dataclass(frozen=True)
class DeployPaths:
    opt_root: Path = Path("/opt/procurali")
    state_dir: Path = Path("/var/lib/procurali-deploy")
    hub_db: Path = Path("/var/lib/procurali-hub/hub.db")
    hub_backups_dir: Path = Path("/var/lib/procurali-hub/backups")
    caddy_main_config: Path = Path("/etc/caddy/Caddyfile")
    caddy_upstream_file: Path = Path("/etc/caddy/procurali-upstream.caddy")
    site_live_link: Path = Path("/srv/procurali/site")

    @property
    def releases_dir(self) -> Path:
        return self.opt_root / "releases"

    @property
    def current_link(self) -> Path:
        return self.opt_root / "current"

    @property
    def active_slot_file(self) -> Path:
        return self.opt_root / "active-slot"

    @property
    def deploy_record_file(self) -> Path:
        return self.state_dir / "current.json"

    def slot_hub_bin(self, slot: str) -> Path:
        return self.opt_root / "slots" / slot / "zaturo-hub"


def other_slot(slot: str) -> str:
    if slot not in SLOTS:
        raise ValueError(f"unsupported slot: {slot}")
    return "b" if slot == "a" else "a"


def github_get_json(url: str, token: str | None = None, timeout: int = 20) -> Any:
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": USER_AGENT,
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def download_to_file(url: str, dest: Path, token: str | None = None, timeout: int = 60) -> None:
    headers = {"Accept": "application/octet-stream", "User-Agent": USER_AGENT}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, headers=headers)
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_name(f".{dest.name}.part.{os.getpid()}")
    with urllib.request.urlopen(req, timeout=timeout) as resp, tmp.open("wb") as out:
        shutil.copyfileobj(resp, out)
    os.replace(tmp, dest)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        while chunk := fh.read(1024 * 1024):
            h.update(chunk)
    return h.hexdigest()


def verify_ci_checks_passed(
    check_runs_payload: dict[str, Any],
    required_checks: tuple[str, ...] = DEFAULT_REQUIRED_CHECKS,
) -> tuple[bool, str]:
    """Ensures every required GitHub check run completed with `success` on the commit."""
    by_name: dict[str, list[dict[str, Any]]] = {}
    for run in check_runs_payload.get("check_runs") or []:
        name = str(run.get("name") or "").strip()
        if name:
            by_name.setdefault(name, []).append(run)

    for required in required_checks:
        runs = by_name.get(required)
        if not runs:
            return False, f"missing required CI check run: {required}"
        latest = max(runs, key=lambda r: int(r.get("id") or 0))
        if latest.get("status") != "completed" or latest.get("conclusion") != "success":
            return (
                False,
                f"CI check {required!r} is status={latest.get('status')!r} conclusion={latest.get('conclusion')!r}",
            )
    return True, "all required CI check runs succeeded"


def find_latest_procurali_release(releases: list[dict[str, Any]]) -> dict[str, Any] | None:
    for rel in releases:
        if rel.get("draft") or rel.get("prerelease"):
            continue
        tag = str(rel.get("tag_name") or "")
        if not tag.startswith("procurali-"):
            continue
        assets = {str(a.get("name") or ""): a for a in (rel.get("assets") or [])}
        if BUNDLE_ASSET_NAME in assets and MANIFEST_ASSET_NAME in assets:
            return rel
    return None


def safe_extract_tar_gz(archive_path: Path, dest_dir: Path) -> None:
    """Safely extracts a release bundle rejecting traversal, symlinks outside root, and special files."""
    dest_dir.mkdir(parents=True, exist_ok=True)
    root = dest_dir.resolve()
    with tarfile.open(archive_path, "r:gz") as tf:
        for member in tf.getmembers():
            rel = Path(member.name)
            if rel.is_absolute() or ".." in rel.parts:
                raise ValueError(f"unsafe archive path: {member.name}")
            if not (member.isfile() or member.isdir()):
                raise ValueError(f"unsupported archive member type for {member.name}")
            resolved = (root / rel).resolve()
            if resolved != root and root not in resolved.parents:
                raise ValueError(f"archive member escapes extraction root: {member.name}")
        tf.extractall(root, filter="data")


def verify_manifest_and_binaries(release_dir: Path, manifest: dict[str, Any]) -> None:
    files = manifest.get("files") or {}
    for rel_path, expected_sha in files.items():
        target = (release_dir / rel_path).resolve()
        if release_dir.resolve() not in target.parents:
            raise ValueError(f"manifest path escapes release directory: {rel_path}")
        if not target.is_file():
            raise FileNotFoundError(f"missing release file: {rel_path}")
        actual = sha256_file(target)
        if actual != str(expected_sha).lower():
            raise ValueError(
                f"sha256 mismatch for {rel_path}: expected {expected_sha}, got {actual}"
            )


def render_caddy_upstream(port: int) -> str:
    return (
        "# Managed automatically by procurali-deploy-agent (blue-green Hub upstream).\n"
        "http://127.0.0.1:7878 {\n"
        f"\treverse_proxy 127.0.0.1:{port}\n"
        "}\n"
    )


def backup_sqlite_db(
    db_path: Path, backups_dir: Path, short_sha: str, keep: int = 5
) -> Path | None:
    if not db_path.is_file():
        return None
    backups_dir.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    dest = backups_dir / f"hub-pre-{short_sha}-{stamp}.db"
    subprocess.run(
        ["sqlite3", str(db_path), f".backup '{dest}'"],
        capture_output=True,
        text=True,
        timeout=30,
        check=True,
    )
    os.chmod(dest, 0o600)
    old_backups = sorted(
        backups_dir.glob("hub-pre-*.db"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    for stale in old_backups[keep:]:
        stale.unlink(missing_ok=True)
    return dest


def atomic_symlink_replace(target: Path, link_path: Path) -> None:
    link_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = link_path.with_name(f".{link_path.name}.next.{os.getpid()}")
    if tmp.exists() or tmp.is_symlink():
        tmp.unlink()
    os.symlink(str(target), str(tmp))
    os.replace(str(tmp), str(link_path))


def wait_http_health(port: int, path: str = "/healthz", retries: int = 30, delay_s: float = 0.5) -> None:
    last_err = "not checked"
    for _ in range(retries):
        conn = None
        try:
            conn = http.client.HTTPConnection("127.0.0.1", port, timeout=2)
            conn.request("GET", path)
            resp = conn.getresponse()
            _ = resp.read()
            if resp.status == 200:
                return
            last_err = f"HTTP {resp.status}"
        except Exception as exc:
            last_err = str(exc)
        finally:
            if conn is not None:
                try:
                    conn.close()
                except Exception:
                    pass
        time.sleep(delay_s)
    raise RuntimeError(f"standby slot 127.0.0.1:{port}{path} failed health check: {last_err}")


def read_active_slot(path: Path) -> str:
    if path.is_file():
        val = path.read_text(encoding="utf-8").strip()
        if val in SLOTS:
            return val
    return "a"


def read_deploy_record(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def write_deploy_record(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(f".{path.name}.tmp.{os.getpid()}")
    tmp.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    os.replace(tmp, path)


def run_cmd(args: list[str], *, check: bool = True, timeout: int = 60) -> subprocess.CompletedProcess[str]:
    return subprocess.run(args, capture_output=True, text=True, timeout=timeout, check=check)


def deploy_blue_green_release(
    paths: DeployPaths,
    manifest: dict[str, Any],
    release_dir: Path,
    *,
    health_check_fn: Callable[[int], None] = wait_http_health,
    cmd_runner: Callable[..., subprocess.CompletedProcess[str]] = run_cmd,
    drain_seconds: float = 2.0,
) -> dict[str, Any]:
    """Performs a zero-downtime slot swap from active -> standby with automatic rollback."""
    commit_sha = str(manifest["commit_sha"])
    short_sha = commit_sha[:12]
    release_tag = str(manifest["release_tag"])
    active = read_active_slot(paths.active_slot_file)
    standby = other_slot(active)
    active_port = SLOTS[active]
    standby_port = SLOTS[standby]

    new_hub = release_dir / "bin" / "zaturo-hub"
    new_worker = release_dir / "bin" / "zaturo-worker"
    for bin_path in (new_hub, new_worker):
        os.chmod(bin_path, 0o755)
        cmd_runner([str(bin_path), "--help"], check=True, timeout=10)

    # Online SQLite snapshot before starting newer Hub schema/binary.
    backup_path = backup_sqlite_db(paths.hub_db, paths.hub_backups_dir, short_sha)

    # Install standby Hub binary atomically before starting standby slot.
    standby_bin = paths.slot_hub_bin(standby)
    standby_bin.parent.mkdir(parents=True, exist_ok=True)
    standby_tmp = standby_bin.with_name(f".zaturo-hub.next.{os.getpid()}")
    shutil.copy2(new_hub, standby_tmp)
    os.chmod(standby_tmp, 0o755)
    os.replace(standby_tmp, standby_bin)

    cmd_runner(["systemctl", "restart", f"procurali-hub@{standby}"], check=True, timeout=30)
    try:
        health_check_fn(standby_port)
    except Exception:
        cmd_runner(["systemctl", "stop", f"procurali-hub@{standby}"], check=False, timeout=20)
        raise

    # Cut Caddy upstream over to healthy standby slot atomically.
    upstream_tmp = paths.caddy_upstream_file.with_name(f".{paths.caddy_upstream_file.name}.next.{os.getpid()}")
    upstream_tmp.parent.mkdir(parents=True, exist_ok=True)
    upstream_tmp.write_text(render_caddy_upstream(standby_port), encoding="utf-8")
    os.chmod(upstream_tmp, stat.S_IRUSR | stat.S_IWUSR | stat.S_IRGRP | stat.S_IROTH)
    try:
        os.replace(upstream_tmp, paths.caddy_upstream_file)
        cmd_runner(
            ["caddy", "validate", "--config", str(paths.caddy_main_config)],
            check=True,
            timeout=20,
        )
        cmd_runner(["systemctl", "reload", "caddy"], check=True, timeout=20)
        health_check_fn(7878)
    except Exception:
        # Roll back Caddy upstream to original healthy slot immediately.
        fallback_tmp = paths.caddy_upstream_file.with_name(f".{paths.caddy_upstream_file.name}.rb.{os.getpid()}")
        fallback_tmp.write_text(render_caddy_upstream(active_port), encoding="utf-8")
        os.replace(fallback_tmp, paths.caddy_upstream_file)
        cmd_runner(["systemctl", "reload", "caddy"], check=False, timeout=20)
        cmd_runner(["systemctl", "stop", f"procurali-hub@{standby}"], check=False, timeout=20)
        raise

    # Update active slot + current release symlink, drain old slot, and refresh auxiliary services.
    slot_tmp = paths.active_slot_file.with_name(f".active-slot.tmp.{os.getpid()}")
    slot_tmp.write_text(f"{standby}\n", encoding="utf-8")
    os.replace(slot_tmp, paths.active_slot_file)
    atomic_symlink_replace(release_dir, paths.current_link)

    if drain_seconds > 0:
        time.sleep(drain_seconds)
    cmd_runner(["systemctl", "enable", f"procurali-hub@{standby}"], check=False, timeout=20)
    cmd_runner(["systemctl", "disable", f"procurali-hub@{active}"], check=False, timeout=20)
    cmd_runner(["systemctl", "stop", f"procurali-hub@{active}"], check=False, timeout=20)

    # Copy updated worker/site-sync helper into canonical /opt/procurali/bin/ locations if present
    # and refresh site-sync + worker + telegram poller safely behind the live site symlink.
    bin_dir = paths.opt_root / "bin"
    if bin_dir.is_dir():
        worker_dest = bin_dir / "zaturo-worker"
        worker_tmp = bin_dir / f".zaturo-worker.next.{os.getpid()}"
        shutil.copy2(new_worker, worker_tmp)
        os.chmod(worker_tmp, 0o755)
        os.replace(worker_tmp, worker_dest)

        sync_src = release_dir / "deploy" / "procurali" / "procurali_site_sync.py"
        sync_dest = bin_dir / "procurali-site-sync.py"
        if sync_src.is_file():
            sync_tmp = bin_dir / f".procurali-site-sync.py.next.{os.getpid()}"
            shutil.copy2(sync_src, sync_tmp)
            os.chmod(sync_tmp, 0o755)
            os.replace(sync_tmp, sync_dest)

    cmd_runner(
        [
            "systemctl",
            "restart",
            "procurali-worker",
            "procurali-telegram-poll",
            "procurali-site-sync",
        ],
        check=True,
        timeout=30,
    )

    record = {
        "status": "ok",
        "commit_sha": commit_sha,
        "release_tag": release_tag,
        "active_slot": standby,
        "previous_slot": active,
        "release_dir": str(release_dir),
        "backup_path": str(backup_path) if backup_path else None,
        "deployed_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    write_deploy_record(paths.deploy_record_file, record)
    prune_old_releases(paths.releases_dir, current_link=paths.current_link)
    return record


def prune_old_releases(
    releases_dir: Path, *, current_link: Path, keep: int = 5
) -> list[Path]:
    if not releases_dir.is_dir():
        return []
    live = current_link.resolve() if current_link.exists() else None
    dirs = sorted(
        (p for p in releases_dir.iterdir() if p.is_dir()),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    removed: list[Path] = []
    for stale in dirs[keep:]:
        if live is not None and stale.resolve() == live:
            continue
        shutil.rmtree(stale, ignore_errors=True)
        removed.append(stale)
    return removed


def run_once(
    *,
    repo: str,
    paths: DeployPaths,
    required_checks: tuple[str, ...],
    dry_run: bool = False,
) -> dict[str, Any]:
    gh_token = os.environ.get("GITHUB_TOKEN", "").strip() or None
    releases = github_get_json(
        f"https://api.github.com/repos/{repo}/releases?per_page=10",
        token=gh_token,
    )
    rel = find_latest_procurali_release(releases)
    if rel is None:
        return {"status": "idle", "reason": "no_procurali_release_found"}

    assets = {str(a["name"]): a for a in rel.get("assets") or []}
    manifest_tmp = paths.state_dir / "manifest.download.json"
    download_to_file(
        str(assets[MANIFEST_ASSET_NAME]["browser_download_url"]),
        manifest_tmp,
        token=gh_token,
    )
    manifest = json.loads(manifest_tmp.read_text(encoding="utf-8"))
    commit_sha = str(manifest.get("commit_sha") or "").strip()
    if len(commit_sha) < 12:
        raise ValueError("manifest missing valid commit_sha")

    current_rec = read_deploy_record(paths.deploy_record_file)
    if current_rec and current_rec.get("status") == "ok" and current_rec.get("commit_sha") == commit_sha:
        return {
            "status": "up_to_date",
            "commit_sha": commit_sha,
            "release_tag": rel.get("tag_name"),
        }

    check_runs = github_get_json(
        f"https://api.github.com/repos/{repo}/commits/{commit_sha}/check-runs?per_page=100",
        token=gh_token,
    )
    passed, reason = verify_ci_checks_passed(check_runs, required_checks=required_checks)
    if not passed:
        return {
            "status": "waiting_ci",
            "commit_sha": commit_sha,
            "release_tag": rel.get("tag_name"),
            "reason": reason,
        }

    if dry_run:
        return {
            "status": "ready_dry_run",
            "commit_sha": commit_sha,
            "release_tag": rel.get("tag_name"),
            "reason": reason,
        }

    archive_tmp = paths.state_dir / f"{commit_sha[:12]}-{BUNDLE_ASSET_NAME}"
    download_to_file(
        str(assets[BUNDLE_ASSET_NAME]["browser_download_url"]),
        archive_tmp,
        token=gh_token,
    )
    expected_bundle_sha = str(manifest.get("bundle_sha256") or "").lower()
    if expected_bundle_sha:
        actual_bundle_sha = sha256_file(archive_tmp)
        if actual_bundle_sha != expected_bundle_sha:
            raise ValueError(
                f"bundle sha256 mismatch: expected {expected_bundle_sha}, got {actual_bundle_sha}"
            )

    release_dir = paths.releases_dir / commit_sha[:12]
    staging_dir = paths.releases_dir / f".staging-{commit_sha[:12]}-{os.getpid()}"
    if staging_dir.exists():
        shutil.rmtree(staging_dir, ignore_errors=True)
    safe_extract_tar_gz(archive_tmp, staging_dir)
    verify_manifest_and_binaries(staging_dir, manifest)
    if release_dir.exists():
        shutil.rmtree(release_dir, ignore_errors=True)
    os.replace(staging_dir, release_dir)
    archive_tmp.unlink(missing_ok=True)

    result = deploy_blue_green_release(paths, manifest, release_dir)
    log("procurali_deployed", **result)
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Procurali zero-downtime pull auto-deploy agent")
    parser.add_argument("--repo", default=os.environ.get("PROCURALI_GITHUB_REPO", DEFAULT_REPO))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--opt-root", default="/opt/procurali")
    parser.add_argument("--state-dir", default="/var/lib/procurali-deploy")
    args = parser.parse_args(argv)

    checks_env = os.environ.get("PROCURALI_REQUIRED_CHECKS", "")
    required_checks = (
        tuple(c.strip() for c in checks_env.split(",") if c.strip())
        if checks_env.strip()
        else DEFAULT_REQUIRED_CHECKS
    )
    paths = DeployPaths(opt_root=Path(args.opt_root), state_dir=Path(args.state_dir))
    try:
        out = run_once(
            repo=args.repo,
            paths=paths,
            required_checks=required_checks,
            dry_run=args.dry_run,
        )
        print(json.dumps(out, ensure_ascii=False), flush=True)
        return 0
    except urllib.error.URLError as exc:
        log("deploy_network_error", error=str(exc))
        return 1
    except Exception as exc:
        log("deploy_failed", error=str(exc))
        return 1


if __name__ == "__main__":
    sys.exit(main())
