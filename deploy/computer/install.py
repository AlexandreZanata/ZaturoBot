#!/usr/bin/env python3
"""Plan/apply an additive local computer bridge installation; all profile grants start disabled.

No network, package manager, desktop setup, service restart, account or credential access.
An optional official backend artifact must already have been obtained and verified separately.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import stat
import tempfile
import time
import tomllib

import linux_mcp as bridge

SOURCE_ROOT = Path(__file__).resolve().parents[2]
HELPERS = ("linux_mcp.py", "browser_mcp.py", "bitlupa_service.py", "social_service.py", "wayland_capture.py")
LICENSE_SHA256 = "865367984df228c2bbf9df7e790b26c3b083376b705375db36a8a032daa03a25"
RELEASE_HASHES = {
    "x86_64": {"gnu": "9105b0caefe67881bd9549a51c59252d2f1dfe4306d9cc71be0091cd3085d122", "musl": "5755478bd24c31eef14612a19e630c856c249052f0e9277ee972b60138efb611"},
    "aarch64": {"gnu": "296c1c2f26ac595a7a92ddae0b086c6edd9528b1d8f25e9c868e7908bbf2a9cd", "musl": "d112d6e7377b057b5d6e50fe691709720df183a7bd6149a47c2688929135d9fa"},
}
STOP_CONTENT = b"ZaturoBot local emergency stop. Remove only on the owner's explicit resume request.\n"


def safe(path):
    path = Path(path)
    bridge.need(path.is_absolute() and ".." not in path.parts and not any(p.is_symlink() for p in [path, *path.parents]), "unsafe_install_path")
    return path


def directory(path, apply=False, private=True):
    path = safe(path)
    if path.exists():
        info = path.stat()
        bridge.need(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & (0o077 if private else 0o022), "unsafe_install_directory")
    elif apply:
        # Path.mkdir(parents=True) leaves intermediate modes dependent on the
        # caller's umask. Every directory we add is private, without chmod of an
        # existing owner directory or any change to its existing contents.
        missing = []; parent = path
        while not parent.exists():
            missing.append(parent); parent = parent.parent
        bridge.need(parent.is_dir(), "unsafe_install_directory")
        for item in reversed(missing): item.mkdir(mode=0o700)
    return path


def regular(path, *, private=False, reviewed_source=False, maximum=128*1024):
    path = safe(path)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        # Git checkouts on this machine use the owner's shared group (0664). Reading
        # reviewed source does not execute it; installed artifacts are always stricter.
        denied_mode = 0o077 if private else 0o002 if reviewed_source else 0o022
        owners = (os.getuid(),) if reviewed_source else (os.getuid(), 0)
        bridge.need(stat.S_ISREG(info.st_mode) and info.st_uid in owners and not info.st_mode & denied_mode and info.st_size <= maximum, "unsafe_install_file")
        data = stream.read(maximum+1)
        bridge.need(len(data) <= maximum, "install_file_too_large")
        return data


def atomic(path, data, mode):
    safe(path); directory(path.parent, True)
    if path.exists(): regular(path, maximum=32*1024*1024)
    fd, temporary = tempfile.mkstemp(prefix=".zaturo-computer-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as out:
            os.fchmod(out.fileno(), mode); out.write(data); out.flush(); os.fsync(out.fileno())
        os.replace(temporary, path)
        directory_fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try: os.fsync(directory_fd)
        finally: os.close(directory_fd)
    finally:
        if os.path.exists(temporary): os.unlink(temporary)


def quoted(value): return json.dumps(str(value), ensure_ascii=False)


def templates(home, binary, binary_hash):
    state = home/".local/state/zaturobot/computer"
    common = "# Explicit local grants only; installation never enables desktop observation/input.\nenabled = false\nallow_input = false\nallow_capture = false\ntimeout_seconds = 15\n"
    linux = common + "binary = "+quoted(binary)+"\nbinary_sha256 = "+quoted(binary_hash)+"\nstate_dir = "+quoted(state/"desktop")+"\nallowed_apps = []\n"
    browser = common + "node = \"/absolute/path/to/verified/node\"\nnode_sha256 = \"\"\nplaywright_cli = \"/absolute/path/to/pinned/node_modules/@playwright/mcp/cli.js\"\nplaywright_cli_sha256 = \"\"\nplaywright_tree_sha256 = \"\"\nstate_dir = "+quoted(state/"browser")+"\ncdp_endpoint = \"\"\nprofile_dir = "+quoted(home/".local/share/zaturobot/browser-automation")+"\nbrowser_pid = 0\nallowed_origins = []\n"
    return {"computer-linux.toml": linux.encode(), "computer-browser.toml": browser.encode()}


def install(home, *, apply=False, source_root=SOURCE_ROOT, backend_binary=None, backend_sha256=None, backend_license=None, pause=None, resume=None):
    home = safe(Path(home))
    bridge.need(home.is_dir() and home.stat().st_uid == os.getuid(), "owner_home_required")
    bridge.need(pause in (None, "desktop", "browser") and resume in (None, "desktop", "browser") and not (pause and resume), "invalid_stop_request")
    machine = platform.machine()
    hashes = RELEASE_HASHES.get(machine)
    bridge.need(hashes is not None, "unsupported_backend_architecture")
    lib = directory(home/".local/lib/zaturobot/computer", False)
    config = directory(home/".config/zaturobot", False)
    state = directory(home/".local/state/zaturobot/computer", False)
    installed_backend = home/".local/lib/zaturobot/backends/computer-use-linux"/bridge.UPSTREAM_VERSION/"computer-use-linux"
    files = {}
    for name in HELPERS:
        data = regular(Path(source_root)/"deploy/computer"/name, reviewed_source=True)
        bridge.need(b"\0" not in data, "invalid_helper_source")
        files[lib/name] = (data, 0o700)
    backend_info = {"version": bridge.UPSTREAM_VERSION, "source_commit": bridge.UPSTREAM_COMMIT, "project": bridge.UPSTREAM_URL, "release_url": "https://github.com/agent-sh/computer-use-linux/releases/tag/v"+bridge.UPSTREAM_VERSION, "architecture": machine, "official_sha256": hashes, "installed": False, "artifact_copy_requested": backend_binary is not None}
    backend_hash = hashes["gnu"]
    if any(v is not None for v in (backend_binary, backend_sha256, backend_license)):
        bridge.need(all(v is not None for v in (backend_binary, backend_sha256, backend_license)), "complete_verified_artifact_required")
        bridge.need(backend_sha256 in hashes.values(), "official_artifact_hash_required")
        data = regular(backend_binary, maximum=32*1024*1024)
        bridge.need(hashlib.sha256(data).hexdigest() == backend_sha256, "artifact_hash_mismatch")
        license_data = regular(backend_license, maximum=16384)
        bridge.need(hashlib.sha256(license_data).hexdigest() == LICENSE_SHA256, "upstream_license_mismatch")
        files[installed_backend] = (data, 0o700); backend_hash = backend_sha256
        license_dir = home/".local/share/zaturobot/licenses/computer-use-linux"/bridge.UPSTREAM_VERSION
        files[license_dir/"LICENSE"] = (license_data, 0o600)
        files[license_dir/"NOTICE.json"] = (json.dumps({"project": "https://github.com/agent-sh/computer-use-linux", "version": bridge.UPSTREAM_VERSION, "source_commit": bridge.UPSTREAM_COMMIT, "license": "MIT", "license_sha256": LICENSE_SHA256, "artifact_sha256": backend_sha256}, indent=2).encode()+b"\n", 0o600)
        backend_info.update(sha256=backend_sha256, destination=str(installed_backend), license=str(license_dir/"LICENSE"))
    elif installed_backend.exists():
        current = regular(installed_backend, maximum=32*1024*1024); actual = hashlib.sha256(current).hexdigest()
        bridge.need(actual in hashes.values(), "unreviewed_existing_backend")
        backend_hash = actual; backend_info.update(installed=True, sha256=actual, destination=str(installed_backend))
    profiles = templates(home, installed_backend, backend_hash)
    preserved = []
    for name, body in profiles.items():
        destination = config/name
        if destination.exists():
            regular(destination, private=True)
            preserved.append(str(destination))
        else: files[destination] = (body, 0o600)
    # Validate every destination before creating directories or replacing a single file.
    changed = []
    for path, (data, mode) in files.items():
        safe(path)
        directory(path.parent, False)
        if path.exists():
            current = regular(path, maximum=32*1024*1024)
            current_mode = stat.S_IMODE(path.stat().st_mode)
            if current != data or current_mode != mode: changed.append((path, current, current_mode))
    backend_state = state/(pause or resume) if pause or resume else None
    if backend_state is not None:
        # Existing profiles are otherwise opaque/preserved, but a local stop must
        # address the state directory the selected bridge actually uses.
        profile = config/("computer-linux.toml" if (pause or resume) == "desktop" else "computer-browser.toml")
        if profile.exists():
            try: settings = tomllib.loads(regular(profile, private=True).decode())
            except (ValueError, UnicodeError): raise bridge.Rejected("invalid_stop_profile") from None
            bridge.need(isinstance(settings.get("state_dir"), str), "invalid_stop_profile")
            backend_state = safe(Path(settings["state_dir"]))
        directory(backend_state, False)
        marker = backend_state/"STOP"
        if marker.exists() or marker.is_symlink():
            regular(marker, private=True, maximum=4096)
    report = {
        "schema": 1, "mode": "plan", "home": str(home),
        "helpers": [{"path": str(lib/name), "sha256": hashlib.sha256(files[lib/name][0]).hexdigest()} for name in HELPERS],
        "profiles": [str(config/name) for name in profiles], "existing_profiles_preserved": preserved,
        "backend": backend_info, "credentials_read_or_changed": False, "worker_config_changed": False,
        "grants_enabled_by_installer": False, "new_profiles_disabled": True, "desktop_or_accounts_contacted": False,
        "stop_action": {"backend": pause or resume, "action": "pause" if pause else "resume" if resume else None, "marker": str(backend_state/"STOP") if backend_state is not None else None},
        "prerequisites": ["Linux x86_64/aarch64; Python3.11+ with pidfd and private user-owned paths.", "An explicitly selected desktop application and locally reviewed allow_input/allow_capture grants; no application is inferred.", "AT-SPI2 session bus and accessibility support. On Ubuntu/Debian relevant packages are at-spi2-core and desktop-specific portal support; install only under a separate owner-approved OS setup.", "X11 keyboard fallback needs xdotool and a valid display. Wayland support depends on the actual compositor/portal; doctor detection is not capture/input acceptance.", "The verified computer-use-linux release artifact plus its pinned MIT license. This installer does not download or execute it.", "Optional browser backend: exact @playwright/mcp0.0.85 and Playwright/core1.66.0-alpha-2026-11-09, verified Node/cli.js and full package-tree hashes, a dedicated private Chrome/Chromium profile and loopbackCDP PID marker. No normal or YoutubeBot profile attachment.", "Both generated profiles stay disabled. Review/update the local profile and use the bounded native owner-approval flow before actual actions."],
        "playwright": {"package": "@playwright/mcp", "version": "0.0.85", "source_commit": "4e0794dd0a0176519830a5ed27b491137cf9338f", "license": "Apache-2.0", "staging_policy": "Exact package pin, npm --ignore-scripts --no-audit --no-fund; no browser download/launch; verify package-lock integrity separately."},
        "diagnostic_commands": [["python3", str(lib/"linux_mcp.py"), "--config", str(config/"computer-linux.toml"), "doctor"], ["python3", str(lib/"browser_mcp.py"), "--config", str(config/"computer-browser.toml"), "doctor"]],
    }
    report["prerequisites"].insert(4, "Private PC screenshots use either the fixed native X11 path or a separately explicit worker wayland_portal_capture grant. The latter requires PyGObject/GIO, GStreamer pipewiresrc/pngenc/appsink and native single-window ScreenCast consent for each capture. Never combine targets or fall back to upstream temporary-file screenshot routes.")
    if not apply: return report
    directory(lib, True); directory(config, True); directory(state, True)
    backup = None
    if changed:
        backup = directory(home/".local/state/zaturobot/computer-install-backups"/(str(time.time_ns())+"-"+str(os.getpid())), True)
        manifest=[]
        for index, (path, body, mode) in enumerate(changed):
            name=f"{index:03d}-{hashlib.sha256(body).hexdigest()[:12]}";atomic(backup/name,body,0o600)
            manifest.append({"path":str(path),"backup":name,"mode":mode,"sha256":hashlib.sha256(body).hexdigest()})
        atomic(backup/"manifest.json",json.dumps(manifest,indent=2).encode()+b"\n",0o600)
    for path, (data, mode) in files.items():
        if not path.exists() or regular(path, maximum=32*1024*1024) != data or stat.S_IMODE(path.stat().st_mode) != mode:
            atomic(path, data, mode)
    if backend_state is not None:
        directory(backend_state, True); marker = backend_state/"STOP"
        if pause and not marker.exists(): atomic(marker, STOP_CONTENT, 0o600)
        elif resume and marker.exists():
            regular(marker, private=True, maximum=4096); marker.unlink()
    report["mode"] = "applied"; report["backup"] = str(backup) if backup else None
    if backend_binary is not None: report["backend"]["installed"] = True
    return report


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--home",type=Path,default=Path.home());parser.add_argument("--apply",action="store_true")
    parser.add_argument("--backend-binary",type=Path);parser.add_argument("--backend-sha256");parser.add_argument("--backend-license",type=Path)
    marker=parser.add_mutually_exclusive_group();marker.add_argument("--pause",choices=["desktop","browser"]);marker.add_argument("--resume",choices=["desktop","browser"])
    options=parser.parse_args();os.umask(0o077)
    try:
        report=install(options.home,apply=options.apply,backend_binary=options.backend_binary,backend_sha256=options.backend_sha256,backend_license=options.backend_license,pause=options.pause,resume=options.resume)
        print(json.dumps(report,indent=2,ensure_ascii=False));return 0
    except (bridge.Rejected,OSError,ValueError,KeyError,TypeError) as error:
        print(json.dumps({"schema":1,"mode":"blocked","code":error.code if isinstance(error,bridge.Rejected) else "installation_unavailable","summary":"Installation rejected; preserve existing profiles and inspect private path/artifact permissions without displaying credentials."}));return 1


if __name__=="__main__":raise SystemExit(main())
