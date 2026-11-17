#!/usr/bin/env python3
"""Watches ZaturoBot Hub's ETag-cached site-catalog and publishes static builds atomically.

The builder is the Procurali Next.js storefront packaged by `package_site_builder.py`:
`<site-builder>/src/build.mjs` runs `next build` (static export) for each catalog snapshot,
preferably with the Node runtime bundled at `<site-builder>/runtime/node`.
"""

from __future__ import annotations

import hashlib
import http.client
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.parse
from pathlib import Path

BUILDER_SCRIPT = Path(
    os.environ.get(
        "PROCURALI_SITE_BUILDER",
        "/opt/procurali/current/site-builder/src/build.mjs",
    )
)
STATE_DIR = Path(os.environ.get("PROCURALI_SITE_STATE_DIR", "/var/lib/procurali-site-sync"))
SNAPSHOT_PATH = STATE_DIR / "site-catalog.latest.json"
ETAG_PATH = STATE_DIR / "last.etag"
BUILDER_DIGEST_PATH = STATE_DIR / "builder.sha256"
RELEASES_DIR = Path(os.environ.get("PROCURALI_SITE_RELEASES_DIR", "/srv/procurali/releases"))
LIVE_SITE_LINK = Path(os.environ.get("PROCURALI_LIVE_SITE_DIR", "/srv/procurali/site"))
KEEP_RELEASES = max(2, int(os.environ.get("PROCURALI_SITE_KEEP_RELEASES", "5")))
POLL_SECS = max(2, int(os.environ.get("PROCURALI_SITE_SYNC_POLL_SECS", "5")))
BUILD_TIMEOUT_SECS = max(60, int(os.environ.get("PROCURALI_SITE_BUILD_TIMEOUT_SECS", "900")))
MAX_FAILURE_BACKOFF_SECS = 900

# Paths below the builder package root that builds generate or that never affect output.
DIGEST_EXCLUDED_TOP_LEVEL = {
    ".generated",
    ".next",
    "node_modules",
    "out",
    "public",
    "runtime",
    "next-env.d.ts",
}
DIGEST_EXCLUDED_PREFIXES = ("out-", ".next-")


def log(msg: str, **fields: object) -> None:
    print(json.dumps({"event": msg, **fields}, ensure_ascii=False), flush=True)


def builder_package_root(builder_path: Path = BUILDER_SCRIPT) -> Path:
    """`<root>/src/build.mjs` -> `<root>`, resolving the `/opt/procurali/current` symlink."""
    return builder_path.resolve().parent.parent


def compute_builder_digest(builder_path: Path = BUILDER_SCRIPT) -> str:
    """Hashes the builder sources (app, data, brand, lockfile...) but not generated output.

    A new release with a changed storefront therefore forces a rebuild even when the Hub
    catalog ETag is unchanged, while repeated in-place builds do not.
    """
    root = builder_package_root(builder_path)
    h = hashlib.sha256()
    if not root.is_dir():
        return h.hexdigest()
    for dirpath, dirnames, filenames in os.walk(root):
        current = Path(dirpath)
        if current == root:
            dirnames[:] = [
                d
                for d in dirnames
                if d not in DIGEST_EXCLUDED_TOP_LEVEL and not d.startswith(DIGEST_EXCLUDED_PREFIXES)
            ]
            filenames = [f for f in filenames if f not in DIGEST_EXCLUDED_TOP_LEVEL]
        dirnames.sort()
        for name in sorted(filenames):
            file_path = current / name
            if not file_path.is_file():
                continue
            h.update(file_path.relative_to(root).as_posix().encode("utf-8"))
            h.update(b"\0")
            h.update(file_path.read_bytes())
    return h.hexdigest()


def builder_environment(state_dir: Path = STATE_DIR) -> dict[str, str]:
    """Environment for `next build` inside the hardened unit (ProtectSystem=strict, ProtectHome).

    Only the state directory is writable, so the read-only release is copied there, and the
    temporary files and Node's home directory live there as well.
    """
    work_root = state_dir / "builder"
    env = dict(os.environ)
    # The snapshot is already fetched; the build never talks to the Hub or sees its token.
    for key in ("PROCURALI_HUB_TOKEN", "PROCURALI_HUB_API_URL", "PROCURALI_API_SNAPSHOT"):
        env.pop(key, None)
    env.update(
        {
            "NEXT_TELEMETRY_DISABLED": "1",
            "PROCURALI_SITE_WORK_DIR": str(work_root / "work"),
            "TMPDIR": str(work_root / "tmp"),
            "HOME": str(work_root / "home"),
            "PROCURALI_BUILD_CPUS": os.environ.get("PROCURALI_BUILD_CPUS", "2"),
        }
    )
    for key in ("TMPDIR", "HOME"):
        Path(env[key]).mkdir(parents=True, exist_ok=True)
    return env


def resolve_node_executable(builder_path: Path = BUILDER_SCRIPT) -> str:
    """Prefers PROCURALI_SITE_NODE, then the runtime bundled with the release, then PATH."""
    explicit = os.environ.get("PROCURALI_SITE_NODE", "").strip()
    if explicit:
        return explicit
    bundled = builder_package_root(builder_path) / "runtime" / "node"
    if bundled.is_file() and os.access(bundled, os.X_OK):
        return str(bundled)
    return "node"


def verify_built_site(site_dir: Path) -> None:
    required = [
        site_dir / "index.html",
        site_dir / "ofertas" / "index.html",
        site_dir / "api" / "catalog.json",
        site_dir / "api" / "whatsapp-assisted-drafts.json",
        site_dir / "sitemap.xml",
    ]
    for path in required:
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f"built static site missing required artifact: {path}")
    next_assets = site_dir / "_next" / "static"
    if not next_assets.is_dir() or not any(next_assets.iterdir()):
        raise RuntimeError(f"built static site missing Next.js assets: {next_assets}")


def atomic_symlink_swap(target_dir: Path, link_path: Path) -> None:
    """Atomically swaps `link_path -> target_dir` without deleting the live tree first."""
    link_path.parent.mkdir(parents=True, exist_ok=True)
    if link_path.exists() and not link_path.is_symlink():
        # One-time migration from legacy plain directory: preserve fallback rename until swapped.
        legacy_backup = link_path.with_name(f".{link_path.name}.legacy-{os.getpid()}")
        link_path.rename(legacy_backup)
    tmp_link = link_path.with_name(f".{link_path.name}.next-{os.getpid()}")
    if tmp_link.exists() or tmp_link.is_symlink():
        tmp_link.unlink()
    os.symlink(str(target_dir), str(tmp_link))
    os.replace(str(tmp_link), str(link_path))


def prune_old_releases(
    releases_dir: Path, live_target: Path | None = None, keep: int = KEEP_RELEASES
) -> list[Path]:
    if not releases_dir.is_dir():
        return []
    dirs = sorted(
        (p for p in releases_dir.iterdir() if p.is_dir()),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    removed: list[Path] = []
    for stale in dirs[keep:]:
        if live_target is not None and stale.resolve() == live_target.resolve():
            continue
        shutil.rmtree(stale, ignore_errors=True)
        removed.append(stale)
    return removed


def load_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8").strip() or None
    except FileNotFoundError:
        return None


def save_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(f".{path.name}.tmp.{os.getpid()}")
    tmp.write_text(f"{value}\n", encoding="utf-8")
    os.replace(tmp, path)


def rebuild_site(
    body_bytes: bytes,
    etag: str,
    builder_digest: str,
    *,
    builder_script: Path = BUILDER_SCRIPT,
    releases_dir: Path = RELEASES_DIR,
    live_site_link: Path = LIVE_SITE_LINK,
    state_dir: Path = STATE_DIR,
    chown_caddy: bool = True,
) -> Path:
    state_dir.mkdir(parents=True, exist_ok=True)
    snap_path = state_dir / "site-catalog.latest.json"
    snap_tmp = snap_path.with_name(f".{snap_path.name}.tmp.{os.getpid()}")
    snap_tmp.write_bytes(body_bytes)
    os.replace(snap_tmp, snap_path)

    release_id = f"{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())}-{os.getpid()}"
    target_dir = releases_dir / release_id
    releases_dir.mkdir(parents=True, exist_ok=True)

    try:
        proc = subprocess.run(
            [
                resolve_node_executable(builder_script),
                str(builder_script.resolve()),
                "--api-snapshot",
                str(snap_path),
                "--out-dir",
                str(target_dir),
            ],
            capture_output=True,
            text=True,
            timeout=BUILD_TIMEOUT_SECS,
            check=True,
            env=builder_environment(state_dir),
        )
        verify_built_site(target_dir)
        if chown_caddy:
            subprocess.run(
                ["chown", "-R", "caddy:caddy", str(target_dir)],
                capture_output=True,
                text=True,
                timeout=30,
                check=True,
            )
    except Exception:
        shutil.rmtree(target_dir, ignore_errors=True)
        raise
    atomic_symlink_swap(target_dir, live_site_link)
    prune_old_releases(releases_dir, live_target=target_dir)
    save_text(state_dir / "last.etag", etag)
    save_text(state_dir / "builder.sha256", builder_digest)
    log("site_rebuilt", etag=etag, release=release_id, builder_line=proc.stdout.strip())
    return target_dir


def main() -> int:
    api_url = os.environ["PROCURALI_HUB_API_URL"].rstrip("/")
    token = os.environ["PROCURALI_HUB_TOKEN"].strip()
    parsed = urllib.parse.urlparse(api_url)
    base_path = parsed.path.rstrip("/")
    endpoint_path = f"{base_path}/v1/commerce/mercadolivre/site-catalog"

    STATE_DIR.mkdir(parents=True, exist_ok=True)
    last_etag = load_text(ETAG_PATH)
    last_builder = load_text(BUILDER_DIGEST_PATH)
    consecutive_failures = 0

    while True:
        conn = None
        failed = False
        try:
            current_builder = compute_builder_digest()
            force_rebuild = (
                not (LIVE_SITE_LINK / "index.html").is_file()
                or current_builder != last_builder
            )
            headers = {"Authorization": f"Bearer {token}"}
            if last_etag and not force_rebuild:
                headers["If-None-Match"] = last_etag
            conn = http.client.HTTPConnection(parsed.hostname, parsed.port or 80, timeout=15)
            conn.request("GET", endpoint_path, headers=headers)
            resp = conn.getresponse()
            body = resp.read()
            if resp.status == 304 and not force_rebuild:
                pass
            elif resp.status == 200:
                etag = (resp.getheader("ETag") or f"len-{len(body)}").strip()
                data = json.loads(body)
                if not isinstance(data, dict) or not data.get("offers"):
                    log("site_sync_skipped_empty_catalog", status=resp.status)
                elif etag != last_etag or force_rebuild:
                    # Only build failures back off; Hub/network errors keep the normal poll.
                    failed = True
                    rebuild_site(body, etag, current_builder)
                    failed = False
                    last_etag = etag
                    last_builder = current_builder
            else:
                log("site_sync_unexpected_status", status=resp.status)
        except subprocess.CalledProcessError as exc:
            stderr_tail = (exc.stderr or "").strip()[-400:]
            log("site_sync_error", error=str(exc), stderr=stderr_tail)
        except Exception as exc:
            log("site_sync_error", error=str(exc))
        finally:
            if conn is not None:
                try:
                    conn.close()
                except Exception:
                    pass
        consecutive_failures = consecutive_failures + 1 if failed else 0
        time.sleep(failure_backoff_secs(consecutive_failures))


def failure_backoff_secs(consecutive_failures: int, poll_secs: int = POLL_SECS) -> int:
    """A failing Next.js build is expensive, so retries back off up to 15 minutes."""
    if consecutive_failures <= 0:
        return poll_secs
    return min(MAX_FAILURE_BACKOFF_SECS, max(poll_secs, 30 * 2 ** (consecutive_failures - 1)))


if __name__ == "__main__":
    sys.exit(main())
