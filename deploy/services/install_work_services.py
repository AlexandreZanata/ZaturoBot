#!/usr/bin/env python3
"""Add services/native UI to an enrolled work PC without replacing its identity or journal."""

from __future__ import annotations

import argparse
import copy
import datetime as dt
import fcntl
import hashlib
import json
import os
import re
import shlex
import sqlite3
import stat
import subprocess
import sys
import tempfile
import time
import urllib.parse
import uuid
from pathlib import Path

import tomllib

MAX_CONFIG = 128 * 1024
KNOWN_CAPS = {
    "diagnostic.v1",
    "agent.diagnostic.v1",
    "youtube.read.v1",
    "youtube.submit.v1",
    "android.devices.v1",
    "android.private.v1",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def safe_path(path):
    """Reject symlinks below the caller's resolved home/source boundary."""
    path = Path(path).absolute()
    for parent in [*reversed(path.parents), path]:
        if parent.exists() or parent.is_symlink():
            require(not parent.is_symlink(), "a destination path contains a symlink")
    return path


def regular(path, *, private=False, limit=MAX_CONFIG, read=True):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        require(
            stat.S_ISREG(info.st_mode) and info.st_size <= limit,
            "a bounded regular file is required",
        )
        require(info.st_uid == os.getuid(), "file belongs to another user")
        require(
            info.st_mode & (0o077 if private else 0o022) == 0,
            "file permissions are too broad",
        )
        return stream.read() if read else None


def secure_dir(path):
    safe_path(path)
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    info = path.stat()
    require(
        info.st_uid == os.getuid() and stat.S_ISDIR(info.st_mode),
        "destination directory belongs to another user",
    )
    path.chmod(0o700)
    return path


def atomic_write(path, content, mode=0o600):
    safe_path(path)
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    if path.exists():
        regular(path, limit=256 * 1024**2)
    descriptor, temporary = tempfile.mkstemp(prefix=".zaturo-update-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            os.fchmod(stream.fileno(), mode)
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        parent = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def digest(content):
    return hashlib.sha256(content).hexdigest()


def command(arguments, *, cwd=None, log=None, timeout=60):
    # Do not inherit app/worker/provider/Google credentials into build or pip processes.
    allowed = {
        "PATH",
        "HOME",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "CARGO_BUILD_JOBS",
        "RUSTFLAGS",
        "CC",
        "CXX",
        "PKG_CONFIG_PATH",
    }
    environment = {k: v for k, v in os.environ.items() if k in allowed}
    result = subprocess.run(
        arguments,
        cwd=cwd,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
        check=False,
    )
    if log is not None:
        with open(log, "ab") as stream:
            os.chmod(log, 0o600)
            stream.write(result.stdout)
    require(
        result.returncode == 0,
        "a preparation command failed; inspect the private install log",
    )
    return result.stdout


def source_revision(root, expected):
    require(
        re.fullmatch(r"[0-9a-f]{40}", expected or "") is not None,
        "enrollment must pin a full Git revision",
    )
    revision = command(["git", "rev-parse", "HEAD"], cwd=root).decode().strip()
    require(
        revision == expected, "source revision does not match the private enrollment"
    )
    status = command(
        ["git", "status", "--porcelain", "--untracked-files=normal"], cwd=root
    )
    require(
        not status.strip(), "source checkout has tracked or untracked source changes"
    )
    return revision


def load_enrollment(path):
    value = json.loads(regular(path, private=True, limit=16 * 1024))
    require(
        isinstance(value, dict)
        and set(value)
        == {
            "schema",
            "app_token",
            "expected_zaturo_revision",
            "expected_youtube_revision",
        },
        "invalid work-services enrollment fields",
    )
    token = value.get("app_token")
    require(
        value.get("schema") == 1
        and isinstance(token, str)
        and 32 <= len(token) <= 256
        and all(33 <= ord(c) <= 126 for c in token),
        "invalid application enrollment",
    )
    return value


def endpoint(value):
    parsed = urllib.parse.urlsplit(value)
    require(
        parsed.scheme in ("https", "http")
        and parsed.hostname
        and not parsed.username
        and not parsed.password
        and not parsed.query
        and not parsed.fragment
        and parsed.path in ("", "/"),
        "invalid hub origin",
    )
    require(
        parsed.scheme == "https" or parsed.hostname in ("127.0.0.1", "::1"),
        "HTTP is permitted only on a literal loopback tunnel",
    )
    return value.rstrip("/")


def toml_value(value):
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, int):
        return str(value)
    if isinstance(value, list) and all(not isinstance(item, dict) for item in value):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    raise ValueError("unsupported configuration value; configuration was not replaced")


def render_toml(value):
    lines = ["# Updated additively by the reviewed work-services installer."]

    def table(data, name=""):
        for key, item in data.items():
            require(
                re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", key), "unsupported TOML key"
            )
            if not isinstance(item, dict) and not (
                isinstance(item, list) and item and isinstance(item[0], dict)
            ):
                lines.append(f"{key} = {toml_value(item)}")
        for key, item in data.items():
            qualified = name + "." + key if name else key
            if isinstance(item, dict):
                lines.extend(["", f"[{qualified}]"])
                table(item, qualified)
            elif isinstance(item, list) and item and isinstance(item[0], dict):
                for entry in item:
                    require(isinstance(entry, dict), "invalid mixed TOML array")
                    lines.extend(["", f"[[{qualified}]]"])
                    table(entry, qualified)

    table(value)
    result = ("\n".join(lines) + "\n").encode()
    require(
        tomllib.loads(result.decode()) == value, "configuration serialization failed"
    )
    return result


def update_config(config, youtube, python, options):
    value = copy.deepcopy(config)
    required = {
        "hub_url",
        "worker_id",
        "database",
        "workspace_root",
        "artifact_root",
        "capabilities",
        "max_concurrent_tasks",
        "heartbeat_seconds",
        "reconnect_max_seconds",
    }
    require(
        required <= set(value)
        and set(value) <= required | {"files", "models_config", "youtube", "android"},
        "unsupported existing worker configuration; nothing was replaced",
    )
    require(
        value["worker_id"] == "work-pc",
        "installer requires the already-enrolled work-pc profile",
    )
    endpoint(value["hub_url"])
    require(
        value["max_concurrent_tasks"] == 1, "existing worker concurrency is unsupported"
    )
    require(
        isinstance(value["capabilities"], list)
        and set(value["capabilities"]) <= KNOWN_CAPS
        and "diagnostic.v1" in value["capabilities"],
        "unsupported existing worker capabilities",
    )
    require(
        1 <= value["heartbeat_seconds"] <= 5
        and 1 <= value["reconnect_max_seconds"] <= 60,
        "unsupported existing worker timing",
    )
    require(
        "agent.diagnostic.v1" not in value["capabilities"]
        or value.get("models_config"),
        "existing assisted capability requires its original models configuration",
    )
    for name in ("database", "workspace_root", "artifact_root"):
        require(
            isinstance(value[name], str) and Path(value[name]).is_absolute(),
            "this updater requires existing absolute worker data paths",
        )
    files = value.get("files")
    require(
        isinstance(files, dict) and set(files) == {"identity", "inbox", "cache"},
        "an already-enrolled private file-transfer profile is required",
    )
    for path in files.values():
        require(
            isinstance(path, str) and Path(path).is_absolute(),
            "invalid existing transfer path",
        )
    require(files["inbox"] != files["cache"], "invalid existing transfer directories")
    # Do not serialize or change the age identity, worker.env, SSH key or models configuration.
    value["youtube"] = {
        "python": str(python),
        "project_root": str(youtube),
        "allow_schedule": bool(options.enable_scheduling),
    }
    caps = [c for c in value["capabilities"] if not c.startswith("youtube.")]
    caps.append("youtube.read.v1")
    if options.enable_scheduling:
        caps.append("youtube.submit.v1")
    if options.enable_private_view:
        require(
            options.android_serial,
            "private views require an explicitly supplied USB serial",
        )
    if options.android_serial:
        require(
            re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", options.android_serial)
            and not options.android_serial.startswith("emulator-"),
            "only an explicit USB serial is permitted",
        )
    adb = Path("/usr/bin/adb")
    if options.android_serial:
        require(
            adb.is_file() and os.access(adb, os.X_OK),
            "/usr/bin/adb must be installed first",
        )
    if adb.is_file() and os.access(adb, os.X_OK):
        android = value.setdefault(
            "android", {"devices": [], "allow_sensitive_views": False}
        )
        require(
            set(android) <= {"executable", "devices", "allow_sensitive_views"},
            "unknown existing Android configuration",
        )
        android["executable"] = str(adb)
        devices = android.setdefault("devices", [])
        require(isinstance(devices, list), "invalid existing device registrations")
        if options.android_serial:
            matching = [
                d for d in devices if d.get("device_id") == options.android_device_id
            ]
            require(
                not matching or matching[0].get("adb_serial") == options.android_serial,
                "refusing to replace an existing Android serial registration",
            )
            require(
                not any(
                    d.get("adb_serial") == options.android_serial
                    and d.get("device_id") != options.android_device_id
                    for d in devices
                ),
                "USB serial is already registered under a different device ID",
            )
            device = (
                matching[0]
                if matching
                else {
                    "device_id": options.android_device_id,
                    "adb_serial": options.android_serial,
                    "display_name": "Owner phone",
                    "capabilities": ["device_status"],
                }
            )
            if not matching:
                devices.append(device)
            if (
                options.enable_private_view
                and "sensitive_screen_view" not in device["capabilities"]
            ):
                device["capabilities"].append("sensitive_screen_view")
        if options.enable_private_view:
            android["allow_sensitive_views"] = True
        if "android.devices.v1" not in caps:
            caps.append("android.devices.v1")
        if options.enable_private_view and "android.private.v1" not in caps:
            caps.append("android.private.v1")
        require(len(devices) <= 8, "too many registered Android devices")
    require(
        len(caps) <= 6 and len(caps) == len(set(caps)),
        "invalid service capability combination",
    )
    order = [
        "diagnostic.v1",
        "agent.diagnostic.v1",
        "youtube.read.v1",
        "youtube.submit.v1",
        "android.devices.v1",
        "android.private.v1",
    ]
    value["capabilities"] = [cap for cap in order if cap in caps]
    return value


def validated_binary(path, expected=None):
    content = regular(path, limit=128 * 1024**2)
    require(os.access(path, os.X_OK), "binary must be executable")
    if expected is not None:
        require(
            re.fullmatch(r"[0-9a-f]{64}", expected) and digest(content) == expected,
            "prebuilt binary SHA-256 does not match the explicit verified hash",
        )
    return content


def database_version(path):
    regular(path, private=True, limit=8 * 1024**3, read=False)
    with sqlite3.connect(path.as_uri() + "?mode=ro", uri=True, timeout=5) as database:
        version = database.execute("PRAGMA user_version").fetchone()[0]
        require(version in (2, 3, 6), "worker journal schema is unsupported")
        require(
            database.execute("PRAGMA quick_check").fetchone()[0] == "ok",
            "worker journal failed quick_check",
        )
    return version


def sqlite_backup(source, target):
    with (
        sqlite3.connect(source.as_uri() + "?mode=ro", uri=True, timeout=5) as original,
        sqlite3.connect(target) as backup,
    ):
        original.backup(backup)
        require(
            backup.execute("PRAGMA integrity_check").fetchone()[0] == "ok",
            "consistent worker journal backup failed integrity_check",
        )
    target.chmod(0o600)


def systemctl(*arguments, allow_failure=False):
    result = subprocess.run(
        ["/usr/bin/systemctl", "--user", *arguments],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        timeout=30,
        check=False,
    )
    require(allow_failure or result.returncode == 0, "user service operation failed")
    return result.returncode == 0


def require_quiesced_config(config):
    # A manually launched worker/file command may outlive or bypass the named user unit.
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit() or int(entry.name) == os.getpid():
            continue
        try:
            arguments = [
                os.fsdecode(arg)
                for arg in (entry / "cmdline").read_bytes().split(b"\0")
                if arg
            ]
            candidates = [
                arguments[index + 1]
                for index, argument in enumerate(arguments[:-1])
                if argument == "--config"
            ]
            candidates += [
                argument.partition("=")[2]
                for argument in arguments
                if argument.startswith("--config=")
            ]
            # Also match literal config arguments in other reviewed launch wrappers.
            if str(config) in arguments:
                raise ValueError(
                    "a live process still uses this worker configuration; wait for it to finish"
                )
            for candidate in candidates:
                path = Path(candidate)
                if not path.is_absolute():
                    path = (entry / "cwd").resolve() / path
                require(
                    path.resolve() != config.resolve(),
                    "a live process still uses this worker configuration; wait for it to finish",
                )
        except (FileNotFoundError, PermissionError, ProcessLookupError):
            pass


def offline_home(home, config):
    if home == Path.home().resolve():
        return False
    require_quiesced_config(config)
    return True


def stop_worker(config):
    systemctl("stop", "zaturo-worker.service")
    require(
        not systemctl(
            "is-active", "--quiet", "zaturo-worker.service", allow_failure=True
        ),
        "worker did not quiesce; binaries and journal were not changed",
    )
    require_quiesced_config(config)


def start_worker_stable():
    systemctl("start", "zaturo-worker.service")
    # Type=simple can report active before immediate config/migration failures.
    for _ in range(4):
        time.sleep(0.5)
        require(
            systemctl(
                "is-active", "--quiet", "zaturo-worker.service", allow_failure=True
            ),
            "updated worker did not stay active; inspect the private user journal",
        )


def prepare_python(options, root, home, log):
    python = root / ".venv" / "bin" / "python"
    if options.skip_python_deps:
        require(
            python.exists() and os.access(python, os.X_OK),
            "--skip-python-deps requires an already prepared project .venv",
        )
        return
    marker = root / ".venv" / ".zaturo-desktop-requirements.sha256"
    requirements_hash = digest(regular(root / "requirements-desktop.txt"))
    if (
        python.exists()
        and marker.exists()
        and regular(marker).decode().strip() == requirements_hash
    ):
        return
    require(
        not python.exists(),
        "existing virtualenv is preserved; verify it and use --skip-python-deps, or prepare a fresh checkout",
    )
    if not python.exists():
        require(
            not (root / ".venv").exists(),
            "refusing to overwrite an incomplete existing virtualenv",
        )
        command(
            [sys.executable, "-m", "venv", str(root / ".venv")], log=log, timeout=120
        )
    # Default CPU wheels avoid silently selecting a CUDA runtime on a new Linux PC.
    command(
        [
            str(python),
            "-m",
            "pip",
            "install",
            "--index-url",
            "https://download.pytorch.org/whl/cpu",
            "torch>=2.6,<3",
        ],
        log=log,
        timeout=1800,
    )
    command(
        [
            str(python),
            "-m",
            "pip",
            "install",
            "-r",
            str(root / "requirements-desktop.txt"),
        ],
        log=log,
        timeout=1800,
    )
    command([str(python), "-m", "pip", "check"], log=log, timeout=120)
    # Read-only service metadata: this cannot launch a browser, generation, upload or schedule.
    command(
        [str(python), "-m", "tools.ytbot.service", "capabilities"], cwd=root, log=log
    )
    atomic_write(marker, (requirements_hash + "\n").encode())


def prepare_extension(root, revision, log):
    marker = root / "dist/.zaturo-extension-source"
    manifest = root / "dist/chrome/manifest.json"
    if (
        marker.exists()
        and manifest.is_file()
        and regular(marker).decode().strip() == revision
    ):
        return
    script = root / "scripts/build-chrome.py"
    regular(script, limit=128 * 1024)
    safe_path(root / "dist")
    command([sys.executable, str(script)], cwd=root, log=log, timeout=120)
    require(manifest.is_file(), "Chrome extension build did not produce a manifest")
    atomic_write(marker, (revision + "\n").encode())


def parser():
    result = argparse.ArgumentParser(description=__doc__)
    result.add_argument("--zaturo-root", type=Path, required=True)
    result.add_argument("--youtube-root", type=Path, required=True)
    result.add_argument("--enrollment", type=Path, required=True)
    result.add_argument("--home", type=Path, default=Path.home())
    result.add_argument(
        "--apply",
        action="store_true",
        help="write the reviewed update; default is read-only planning",
    )
    result.add_argument(
        "--no-start",
        action="store_true",
        help="leave worker stopped; required with an alternate offline home",
    )
    result.add_argument(
        "--enable-scheduling",
        action="store_true",
        help="grant typed scheduled batches; does not submit a job",
    )
    result.add_argument(
        "--android-serial", help="explicit USB serial; never selected automatically"
    )
    result.add_argument("--android-device-id", default="owner-phone")
    result.add_argument(
        "--enable-private-view",
        action="store_true",
        help="explicit local sensitive-screen grant for the supplied USB serial",
    )
    result.add_argument(
        "--skip-python-deps",
        action="store_true",
        help="use an existing verified project virtualenv without pip/network",
    )
    result.add_argument(
        "--skip-build",
        action="store_true",
        help="require explicit verified worker/desktop paths and SHA-256 hashes",
    )
    result.add_argument("--worker-binary", type=Path)
    result.add_argument("--desktop-binary", type=Path)
    result.add_argument("--worker-sha256")
    result.add_argument("--desktop-sha256")
    return result


def install(options):
    os.umask(0o077)
    home = options.home.resolve(strict=True)
    require(home.stat().st_uid == os.getuid(), "home belongs to another user")
    source, youtube = (
        options.zaturo_root.resolve(strict=True),
        options.youtube_root.resolve(strict=True),
    )
    enrollment = load_enrollment(options.enrollment)
    source_revision(source, enrollment["expected_zaturo_revision"])
    source_revision(youtube, enrollment["expected_youtube_revision"])
    require(
        (youtube / "tools/ytbot/service.py").is_file(),
        "pinned YoutubeBot service is missing",
    )
    require(
        re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}", options.android_device_id),
        "invalid local Android device ID",
    )
    config_dir = safe_path(home / ".config/zaturobot")
    require(
        config_dir.is_dir() and config_dir.stat().st_mode & 0o077 == 0,
        "existing ZaturoBot configuration directory must be private (0700)",
    )
    worker_path = config_dir / "worker.toml"
    original = regular(worker_path, private=True)
    config = tomllib.loads(original.decode())
    regular(config_dir / "worker.env", private=True)
    identity = Path(config.get("files", {}).get("identity", ""))
    require(identity.is_absolute(), "existing transfer identity is required")
    regular(
        identity, private=True, limit=16 * 1024
    )  # Validate; never rewrite or copy this key.
    offline = offline_home(home, worker_path)
    require(
        not offline or options.no_start, "an alternate offline home requires --no-start"
    )
    python = youtube / ".venv/bin/python"
    updated = update_config(config, youtube, python, options)
    database = safe_path(Path(config["database"]))
    schema = database_version(database)
    app_path = config_dir / "app.env"
    if app_path.exists():
        previous = regular(app_path, private=True)
        # app.env is exclusively the scoped native credential; never overwrite an admin/provider file.
        require(
            all(
                not line.strip() or line.startswith((b"#", b"ZATURO_APP_TOKEN="))
                for line in previous.splitlines()
            ),
            "existing app.env is not an application-only credential file",
        )
    desktop_config = {
        "hub_url": endpoint(config["hub_url"]),
        "credentials_file": str(app_path),
        "target_worker": "work-pc",
        "inbox_root": config["files"]["inbox"],
        "transfer_identity": config["files"]["identity"],
    }
    if (config_dir / "desktop.toml").exists():
        existing = tomllib.loads(
            regular(config_dir / "desktop.toml", private=True).decode()
        )
        require(
            set(existing) <= set(desktop_config),
            "unknown existing desktop fields; nothing was replaced",
        )
        desktop_config.update(existing)
        desktop_config["credentials_file"] = str(app_path)
        desktop_config["target_worker"] = "work-pc"
        endpoint(desktop_config["hub_url"])
        require(
            desktop_config["inbox_root"] == config["files"]["inbox"]
            and desktop_config["transfer_identity"] == config["files"]["identity"],
            "existing desktop uses different transfer paths; it was preserved",
        )
    lib = home / ".local/lib/zaturobot"
    bin_dir = home / ".local/bin"
    unit = home / ".config/systemd/user/zaturo-worker.service"
    writes = {
        worker_path: (render_toml(updated), 0o600),
        app_path: (
            ("ZATURO_APP_TOKEN=" + enrollment["app_token"] + "\n").encode(),
            0o600,
        ),
        config_dir / "desktop.toml": (render_toml(desktop_config), 0o600),
        home / ".local/share/icons/hicolor/scalable/apps/zaturobot.svg": (
            regular(source / "assets/brand/zaturobot-icon.svg", limit=1024 * 1024),
            0o600,
        ),
    }
    icon_notices = source / "apps/zaturo-desktop/assets/material-symbols-rounded"
    if icon_notices.exists():
        notice_destination = home / ".local/share/zaturobot/licenses/material-symbols-rounded"
        for name in ("LICENSE.txt", "NOTICE.txt", "provenance.json"):
            writes[notice_destination / name] = (
                regular(icon_notices / name, limit=128 * 1024),
                0o600,
            )
    # Existing reviewed worker unit remains authoritative, including local resource settings.
    if unit.exists():
        original_unit = regular(unit, limit=64 * 1024).decode()
        expected_exec = "ExecStart=%h/.local/lib/zaturobot/zaturo-worker --config %h/.config/zaturobot/worker.toml connect"
        exact_exec = (
            "ExecStart="
            + str(lib / "zaturo-worker")
            + " --config "
            + str(worker_path)
            + " connect"
        )
        require(
            any(
                line.strip() in (expected_exec, exact_exec)
                for line in original_unit.splitlines()
            ),
            "existing worker unit has a different executable/configuration; it was preserved",
        )
    else:
        writes[unit] = (
            regular(source / "deploy/notebook/zaturo-worker.user.service"),
            0o600,
        )
    worker_wrapper = bin_dir / "zaturo"
    if not worker_wrapper.exists():
        writes[worker_wrapper] = (
            (
                "#!/bin/sh\nset -eu\nset -a\n. "
                + shlex.quote(str(config_dir / "worker.env"))
                + "\nset +a\nexec "
                + shlex.quote(str(lib / "zaturo-worker"))
                + " --config "
                + shlex.quote(str(worker_path))
                + ' "$@"\n'
            ).encode(),
            0o700,
        )
    else:
        regular(worker_wrapper, limit=64 * 1024)
    desktop_wrapper = bin_dir / "zaturo-desktop"
    writes[desktop_wrapper] = (
        (
            "#!/bin/sh\nset -eu\nexec "
            + shlex.quote(str(lib / "zaturo-desktop"))
            + " --config "
            + shlex.quote(str(config_dir / "desktop.toml"))
            + ' "$@"\n'
        ).encode(),
        0o700,
    )
    # Desktop-entry Exec uses its own quoting grammar, not shell quoting.
    executable = (
        str(desktop_wrapper)
        .replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("`", "\\`")
        .replace("$", "\\$")
        .replace("%", "%%")
    )
    entry = regular(source / "apps/zaturo-desktop/packaging/zaturobot.desktop").decode()
    entry = entry.replace("Exec=zaturo-desktop", 'Exec="' + executable + '"')
    writes[home / ".local/share/applications/zaturobot.desktop"] = (
        entry.encode(),
        0o600,
    )
    for destination in [*writes, lib / "zaturo-worker", lib / "zaturo-desktop"]:
        safe_path(destination)
        if destination.exists():
            regular(destination, limit=128 * 1024**2)
    if options.skip_build:
        require(
            all(
                (
                    options.worker_binary,
                    options.desktop_binary,
                    options.worker_sha256,
                    options.desktop_sha256,
                )
            ),
            "--skip-build requires both binary paths and their verified SHA-256 hashes",
        )
        worker = validated_binary(options.worker_binary, options.worker_sha256)
        desktop = validated_binary(options.desktop_binary, options.desktop_sha256)
        writes[lib / "zaturo-worker"] = (worker, 0o700)
        writes[lib / "zaturo-desktop"] = (desktop, 0o700)
    else:
        require(
            not any(
                (
                    options.worker_binary,
                    options.desktop_binary,
                    options.worker_sha256,
                    options.desktop_sha256,
                )
            ),
            "prebuilt binary options require --skip-build",
        )
    report = {
        "schema": 1,
        "mode": "dry_run",
        "worker_id": "work-pc",
        "journal_schema_before": schema,
        "capabilities": updated["capabilities"],
        "scheduling_granted": options.enable_scheduling,
        "private_screen_granted": options.enable_private_view,
        "offline_home": offline,
        "source_revision": enrollment["expected_zaturo_revision"],
        "youtube_revision": enrollment["expected_youtube_revision"],
        "started": False,
        "identity_preserved": True,
        "journal_path_preserved": True,
    }
    if not options.apply:
        return report
    state = secure_dir(home / ".local/state/zaturobot")
    lock_path = safe_path(state / "work-services-install.lock")
    lock_fd = os.open(lock_path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(lock_fd, "wb") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        log = state / "work-services-install.log"
        safe_path(log)
        if log.exists():
            regular(log, private=True, limit=128 * 1024**2)
        if not options.skip_build:
            command(
                [
                    "cargo",
                    "build",
                    "--release",
                    "--locked",
                    "-p",
                    "zaturo-worker",
                    "-p",
                    "zaturo-desktop",
                ],
                cwd=source,
                log=log,
                timeout=3600,
            )
            for name in ("zaturo-worker", "zaturo-desktop"):
                writes[lib / name] = (
                    validated_binary(source / "target/release" / name),
                    0o700,
                )
        prepare_python(options, youtube, home, log)
        prepare_extension(youtube, enrollment["expected_youtube_revision"], log)
        # Preparation may create ignored artifacts only. Re-check source pins immediately before stopping.
        source_revision(source, enrollment["expected_zaturo_revision"])
        source_revision(youtube, enrollment["expected_youtube_revision"])
        require(
            regular(worker_path, private=True) == original,
            "worker configuration changed during preparation; update cancelled",
        )
        changes = {
            p: value
            for p, value in writes.items()
            if not p.exists()
            or regular(p, limit=128 * 1024**2) != value[0]
            or stat.S_IMODE(p.stat().st_mode) != value[1]
        }
        if not changes and (offline or options.no_start or schema == 6):
            if not offline:
                if options.no_start:
                    stop_worker(worker_path)
                elif not systemctl(
                    "is-active", "--quiet", "zaturo-worker.service", allow_failure=True
                ):
                    start_worker_stable()
                    report["started"] = True
            report["mode"] = "already_current"
            return report
        was_running = False
        if not offline:
            was_running = systemctl(
                "is-active", "--quiet", "zaturo-worker.service", allow_failure=True
            )
            stop_worker(worker_path)
        else:
            offline_home(home, worker_path)
        backup = secure_dir(
            state
            / "work-services-backups"
            / (
                dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ-")
                + uuid.uuid4().hex[:8]
            )
        )
        report["backup"] = str(backup)
        try:
            sqlite_backup(database, backup / "worker.db")
            # Copy only configuration/unit/binaries being changed, plus original worker environment.
            # SSH keys, age identities, provider keys and file inbox contents are never copied.
            backups = set(changes) | {
                worker_path,
                config_dir / "worker.env",
                unit,
                lib / "zaturo-worker",
                lib / "zaturo-desktop",
                config_dir / "desktop.toml",
                app_path,
            }
            manifest = {
                "schema": 1,
                "journal_schema": schema,
                "files": [],
                "source_revision": report["source_revision"],
            }
            for index, path in enumerate(sorted(backups)):
                if not path.exists():
                    continue
                content = regular(path, limit=128 * 1024**2)
                mode = stat.S_IMODE(path.stat().st_mode)
                name = f"file-{index:03d}.backup"
                atomic_write(backup / name, content)
                manifest["files"].append(
                    {
                        "path": str(path),
                        "backup": name,
                        "mode": mode,
                        "sha256": digest(content),
                    }
                )
            atomic_write(
                backup / "manifest.json",
                (json.dumps(manifest, indent=2) + "\n").encode(),
            )
            secure_dir(lib)
            for path, (content, mode) in changes.items():
                atomic_write(path, content, mode)
            if not offline:
                systemctl("daemon-reload")
                if not options.no_start:
                    systemctl("enable", "zaturo-worker.service")
                    start_worker_stable()
                    report["started"] = True
        except Exception:
            # A new worker may have migrated its DB. Never restore an older binary or DB automatically.
            # Keep the worker stopped, intact backups and a concrete manual recovery record.
            if not offline:
                systemctl("stop", "zaturo-worker.service", allow_failure=True)
            atomic_write(
                backup / "UPDATE-INCOMPLETE.txt",
                b"Update did not complete. Worker remains stopped. Inspect manifest and journal schema before restoring any binary. Never replace the live journal without a verified recovery plan.\n",
            )
            raise
        report["mode"] = "applied"
        report["previously_running"] = was_running
        report["changed_files"] = len(changes)
        atomic_write(
            state / "work-services-last-report.json",
            (json.dumps(report, indent=2) + "\n").encode(),
        )
        return report


def main(arguments=None):
    try:
        report = install(parser().parse_args(arguments))
        print(json.dumps(report, indent=2))
        return 0
    except (
        ValueError,
        OSError,
        sqlite3.Error,
        subprocess.SubprocessError,
        tomllib.TOMLDecodeError,
    ) as error:
        # Exception details from parsers/subprocesses may contain private source input.
        message = (
            str(error)
            if isinstance(error, ValueError)
            and not isinstance(error, (json.JSONDecodeError, tomllib.TOMLDecodeError))
            else "update rejected; inspect private paths/permissions and install log"
        )
        print("Work-services update failed: " + message, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
