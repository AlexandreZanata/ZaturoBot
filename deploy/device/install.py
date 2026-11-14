#!/usr/bin/env python3
"""Install a Linux device from an owner-issued private enrollment bundle; never log secrets."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys


def private_dir(path):
    path.mkdir(parents=True, mode=0o700, exist_ok=True)
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_mode & 0o077:
        raise ValueError("installation requires real private directories with mode 0700")


def private_write(path, content, mode=0o600):
    data = content.encode() if isinstance(content, str) else content
    if path.is_symlink():
        raise ValueError("installation refuses symlink destinations")
    if path.exists():
        if not path.is_file() or path.read_bytes() != data:
            raise ValueError("existing device configuration differs; preserve it and review enrollment")
        path.chmod(mode)
        return
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def run(args):
    subprocess.run(args, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--home", type=Path, default=Path.home())
    parser.add_argument("--no-start", action="store_true", help="validate/install files without starting services")
    args = parser.parse_args()
    os.umask(0o077)
    info = args.bundle.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077 or info.st_size > 32768:
        raise ValueError("enrollment bundle must be a private regular file (0600)")
    bundle = json.loads(args.bundle.read_text())
    expected = {"schema", "device_id", "ssh_host", "ssh_port", "ssh_user", "ssh_private_key_b64", "known_hosts", "worker_token", "age_identity", "source_commit"}
    if set(bundle) != expected or bundle["schema"] != 1:
        raise ValueError("unsupported enrollment schema")
    if not re.fullmatch(r"[A-Za-z0-9_-]{1,48}", bundle["device_id"]):
        raise ValueError("invalid device ID")
    if not re.fullmatch(r"[A-Za-z0-9.-]{1,253}", bundle["ssh_host"]) or bundle["ssh_port"] != 22 or bundle["ssh_user"] != "zaturo-link":
        raise ValueError("unexpected SSH enrollment endpoint")
    if not re.fullmatch(r"[A-Za-z0-9_-]{32,256}", bundle["worker_token"]):
        raise ValueError("invalid device credential")
    if len(bundle["age_identity"]) > 256 or not bundle["age_identity"].startswith("AGE-" + "SECRET-KEY-"):
        raise ValueError("invalid local encryption identity")
    # A private prompt pins the exact reviewed source revision. No credentials are in that revision.
    repo = Path(__file__).resolve().parents[2]
    commit = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
    if commit != bundle["source_commit"]:
        raise ValueError("check out the source commit specified by the enrollment bundle")
    version = subprocess.check_output([str(args.binary.resolve()), "--version"], text=True).strip()
    manifest=(repo / "Cargo.toml").read_text().split("[workspace.package]",1)[1].split("\n[",1)[0]
    match=re.search(r'^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"\s*$',manifest,re.MULTILINE)
    if not match or version != "zaturo-worker " + match.group(1):
        raise ValueError("the reviewed workspace worker version is required")
    if not args.home.is_absolute() or any(c.isspace() for c in str(args.home)):
        raise ValueError("an absolute home path without whitespace is required")
    config = args.home / ".config/zaturobot"
    state = args.home / ".local/share/zaturobot"
    binaries = args.home / ".local/lib/zaturobot"
    for directory in [config, state, binaries, state / "inbox", state / "transfer-cache"]:
        private_dir(directory)
    private_write(config / "device_ssh", base64.b64decode(bundle["ssh_private_key_b64"], validate=True))
    known = bundle["known_hosts"]
    if not known.endswith("\n") or len(known.splitlines()) != 1 or not known.startswith(bundle["ssh_host"] + " ssh-ed25519 "):
        raise ValueError("one verified Ed25519 SSH host key is required")
    private_write(config / "known_hosts", known)
    private_write(config / "transfer.identity", bundle["age_identity"].strip() + "\n")
    private_write(config / "worker.env", "ZATURO_WORKER_TOKEN=" + bundle["worker_token"] + "\nRUST_LOG=info\n")
    private_write(config / "tunnel.env", f'ZATURO_DEPLOY_SSH_HOST={bundle["ssh_host"]}\nZATURO_DEPLOY_SSH_PORT=22\nZATURO_DEPLOY_SSH_USER=zaturo-link\nZATURO_DEPLOY_SSH_KEY={config}/device_ssh\nZATURO_DEPLOY_KNOWN_HOSTS={config}/known_hosts\n')
    worker = f'''hub_url = "http://127.0.0.1:17878"
worker_id = "{bundle["device_id"]}"
database = "{state}/worker.db"
workspace_root = "{state}/workspaces"
artifact_root = "{state}/artifacts"
capabilities = ["diagnostic.v1"]
max_concurrent_tasks = 1
heartbeat_seconds = 5
reconnect_max_seconds = 30
[files]
identity = "{config}/transfer.identity"
cache = "{state}/transfer-cache"
inbox = "{state}/inbox"
'''
    private_write(config / "worker.toml", worker)
    target = binaries / "zaturo-worker"
    if target.is_symlink():
        raise ValueError("worker binary cannot be a symlink")
    if target.exists() and hashlib.sha256(target.read_bytes()).digest() != hashlib.sha256(args.binary.read_bytes()).digest():
        raise ValueError("existing installed binary differs; stop and back it up before upgrading")
    if not target.exists():
        shutil.copyfile(args.binary, target)
    target.chmod(0o700)
    units = args.home / ".config/systemd/user"
    units.mkdir(parents=True, exist_ok=True)
    for name in ["zaturo-hub-tunnel", "zaturo-worker"]:
        template = repo / "deploy/notebook" / (name + ".user.service")
        private_write(units / (name + ".service"), template.read_bytes(), 0o644)
    # Create a convenient wrapper which loads only the worker credential; no owner/API keys.
    bin_dir = args.home / ".local/bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    wrapper = f'''#!/bin/sh
set -a
. "{config}/worker.env"
set +a
exec "{target}" --config "{config}/worker.toml" "$@"
'''
    private_write(bin_dir / "zaturo", wrapper, 0o700)
    if not args.no_start:
        run(["systemctl", "--user", "daemon-reload"])
        run(["systemctl", "--user", "enable", "--now", "zaturo-hub-tunnel.service", "zaturo-worker.service"])
        run(["loginctl", "--no-ask-password", "enable-linger", os.environ.get("USER", "")])
        run(["systemctl", "--user", "is-active", "zaturo-hub-tunnel.service", "zaturo-worker.service"])
    print("Installed private device profile for " + bundle["device_id"])
    print("Run ~/.local/bin/zaturo files peers and ~/.local/bin/zaturo task diagnostic")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError, KeyError):
        # Do not interpolate exception messages: malformed credentials may appear in parser errors.
        print("Device installation failed. Check prerequisites, source revision and private file permissions; no secret values were logged.", file=sys.stderr)
        sys.exit(1)
