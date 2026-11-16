#!/usr/bin/env python3
"""Watches ZaturoBot Hub's ETag-cached site-catalog and publishes static builds atomically."""

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


def log(msg: str, **fields: object) -> None:
    print(json.dumps({"event": msg, **fields}, ensure_ascii=False), flush=True)


def compute_builder_digest(builder_path: Path = BUILDER_SCRIPT) -> str:
    src_dir = builder_path.parent
    h = hashlib.sha256()
    if src_dir.is_dir():
        for file_path in sorted(p for p in src_dir.rglob("*") if p.is_file()):
            h.update(file_path.relative_to(src_dir).as_posix().encode("utf-8"))
            h.update(b"\0")
            h.update(file_path.read_bytes())
    return h.hexdigest()


def verify_built_site(site_dir: Path) -> None:
    required = [
        site_dir / "index.html",
        site_dir / "ofertas" / "index.html",
        site_dir / "api" / "catalog.json",
        site_dir / "api" / "whatsapp-assisted-drafts.json",
        site_dir / "assets" / "app.js",
    ]
    for path in required:
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f"built static site missing required artifact: {path}")


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

    proc = subprocess.run(
        ["node", str(builder_script), "--api-snapshot", str(snap_path), "--out-dir", str(target_dir)],
        capture_output=True,
        text=True,
        timeout=60,
        check=True,
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

    while True:
        conn = None
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
                    rebuild_site(body, etag, current_builder)
                    last_etag = etag
                    last_builder = current_builder
            else:
                log("site_sync_unexpected_status", status=resp.status)
        except Exception as exc:
            log("site_sync_error", error=str(exc))
        finally:
            if conn is not None:
                try:
                    conn.close()
                except Exception:
                    pass
        time.sleep(POLL_SECS)


if __name__ == "__main__":
    sys.exit(main())
