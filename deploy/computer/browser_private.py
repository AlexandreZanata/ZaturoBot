#!/usr/bin/python3
"""Native owner URL capture. PNG stdout only; separate browser, no semantic/model output."""
from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import select
import stat
import struct
import subprocess
import sys
import time
import browser_mcp
import linux_mcp as base


def node_argv(config):
    node = base.checked_artifact(config.value["node"], config.value["node_sha256"])
    cli = base.checked_artifact(config.value["playwright_cli"], config.value["playwright_cli_sha256"])
    base.need(browser_mcp.package_tree_sha256(cli) == config.value["playwright_tree_sha256"], "package_tree_hash_mismatch")
    script = Path(__file__).resolve().with_suffix(".mjs")
    info = script.lstat()
    base.need(stat.S_ISREG(info.st_mode) and info.st_uid in {0, os.getuid()} and not info.st_mode & 0o022, "unsafe_capture_helper")
    return [str(node), str(script)]


def private_profile(config):
    directory = Path(config.value["profile_dir"])
    base.need(directory.is_absolute() and not any(p.is_symlink() for p in [directory, *directory.parents]), "dedicated_profile_required")
    info = directory.stat()
    base.need(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & 0o077, "dedicated_profile_required")
    marker = json.loads(base.private_read(directory / ".zaturobot-private-browser"))
    base.need(marker == {"schema": 1, "purpose": "zaturobot-private-page-capture"}, "dedicated_profile_required")
    return directory


def read_exact(stream, size, deadline):
    output = bytearray()
    while len(output) < size:
        remaining = deadline - time.monotonic()
        base.need(remaining > 0 and select.select([stream], [], [], remaining)[0], "capture_timeout")
        block = os.read(stream.fileno(), min(size - len(output), 65536))
        base.need(bool(block), "capture_unavailable")
        output.extend(block)
    return output


def capture(config, request):
    base.need(config.allow_capture and not config.allow_input and request.get("owner_confirmed") is True, "capture_grant_required")
    base.need(request.get("tool") == "private_browser_capture" and set(request.get("arguments", {})) == {"url"}, "invalid_request")
    directory = private_profile(config)
    chrome = base.checked_artifact(config.value["browser_executable"], config.value["browser_sha256"])
    cli = Path(config.value["playwright_cli"])
    core = cli.parents[2] / "playwright-core" / "index.mjs"
    base.need(core.is_file() and chrome.name in {"chrome", "chromium", "chromium-browser"}, "invalid_executable")
    node_argv(config)
    with base.operation(config, request, True) as (end, begin):
        begin()
        payload = {"url": request["arguments"]["url"], "profile": str(directory), "chrome": str(chrome), "module": str(core), "expires_at": end}
        env = {k: v for k, v in os.environ.items() if k in base.ENV_NAMES or k.startswith("LC_")}
        env.update(PATH="/usr/bin:/bin", LANG="C.UTF-8")
        process = subprocess.Popen([sys.executable, str(Path(base.__file__).resolve()), "--internal-guard", str(config.path), "private_browser"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env, cwd=config.state, start_new_session=True, bufsize=0)
        image = bytearray()
        try:
            process.stdin.write(json.dumps(payload).encode() + b"\n"); process.stdin.flush()
            deadline = min(time.monotonic() + config.timeout, time.monotonic() + max(0, (end - time.time() * 1000) / 1000))
            length = struct.unpack(">I", read_exact(process.stdout, 4, deadline))[0]
            base.need(57 <= length <= 8 * 1024 * 1024, "invalid_private_image")
            image = read_exact(process.stdout, length, deadline)
            base.validate_png(image); config.check()
            base.need(int(time.time() * 1000) < end, "expired_lease")
            return image
        except BaseException:
            image[:] = b"\0" * len(image)
            raise
        finally:
            process.stdin.close()
            try: process.wait(timeout=2)
            except subprocess.TimeoutExpired: process.kill(); process.wait(timeout=2)
            process.stdout.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    args = parser.parse_args(); os.umask(0o077)
    image = bytearray()
    try:
        config = base.Config(args.config, backend="private_browser")
        request = json.loads(sys.stdin.buffer.read(8193)); base.need(len(base.encoded(request)) <= 8192, "invalid_request")
        image = capture(config, request)
        sys.stdout.buffer.write(image); sys.stdout.buffer.flush()
        return 0
    except Exception:
        sys.stderr.write("private_browser_capture_unavailable\n")
        return 1
    finally:
        image[:] = b"\0" * len(image)


if __name__ == "__main__":
    raise SystemExit(main())
