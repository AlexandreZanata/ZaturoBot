#!/usr/bin/env python3
"""Fixed Playwright MCP/CDP adapter for one explicitly configured dedicated browser.

No browser install, normal-profile attachment, code evaluation, cookies, files or shell.
Page content remains untrusted data. Origin checks are not a network/OS sandbox.
"""
from __future__ import annotations

import argparse
import hashlib
import ipaddress
import itertools
import json
import os
from pathlib import Path
import re
import socket
import stat
import sys
import urllib.error
import urllib.parse
import urllib.request

import linux_mcp as base

UPSTREAM_VERSION = "0.0.85"
UPSTREAM_COMMIT = "4e0794dd0a0176519830a5ed27b491137cf9338f"
UPSTREAM_URL = "https://github.com/microsoft/playwright-mcp/tree/" + UPSTREAM_COMMIT
PACKAGES = {"@playwright/mcp": UPSTREAM_VERSION, "playwright": "1.66.0-alpha-2026-11-09", "playwright-core": "1.66.0-alpha-2026-11-09"}
# The pinned wrapper delegates its MCP server to playwright-core; initialize reports the core version.
RUNTIME_VERSION = PACKAGES["playwright-core"]
MAX_TREE_FILES = 512
MAX_TREE_DIRECTORIES = 256
MAX_TREE_FILE_BYTES = 8 * 1024 * 1024
MAX_TREE_BYTES = 32 * 1024 * 1024
READ_TOOLS = {"browser_snapshot", "browser_navigate"}
INPUT_TOOLS = {"browser_click", "browser_type", "browser_press_key"}
REF = re.compile(r"^[a-zA-Z][a-zA-Z0-9_-]{0,31}$")
SENSITIVE = re.compile(r"password|senha|passphrase|secret|credential|api[_ -]?key|access_token|authorization|bearer|one.time|verification.code|authentication.code|2fa|código.de", re.I)


def origin(url):
    parsed = urllib.parse.urlsplit(url)
    base.need(parsed.scheme == "https" and parsed.hostname and not parsed.username and not parsed.password and parsed.port in (None, 443), "public_https_required")
    host = parsed.hostname.encode("idna").decode().lower().rstrip(".")
    base.need(re.fullmatch(r"[a-z0-9.-]{1,253}", host) and "." in host and not host.endswith((".local", ".localhost", ".internal", ".invalid")), "public_https_required")
    try: base.need(ipaddress.ip_address(host).is_global, "public_host_required")
    except ValueError: pass
    return "https://" + host


def endpoint(config):
    parsed = urllib.parse.urlsplit(config.value.get("cdp_endpoint", ""))
    base.need(parsed.scheme == "http" and parsed.hostname == "127.0.0.1" and parsed.port and 1024 <= parsed.port <= 65535 and parsed.path in ("", "/") and not parsed.username and not parsed.password and not parsed.query and not parsed.fragment, "literal_loopback_cdp_required")
    return "http://127.0.0.1:" + str(parsed.port)


def browser_grant(config):
    origins = config.value.get("allowed_origins", [])
    base.need(isinstance(origins, list) and 1 <= len(origins) <= 32 and len(set(origins)) == len(origins), "origin_grant_required")
    base.need(all(isinstance(v, str) and origin(v) == v and urllib.parse.urlsplit(v).path == "" for v in origins), "invalid_origin_grant")
    return origins


def browser_identity(config):
    browser_grant(config)
    target = endpoint(config)
    profile = Path(config.value["profile_dir"])
    base.need(profile.is_absolute() and not any(p.is_symlink() for p in [profile, *profile.parents]), "dedicated_profile_required")
    try: info = profile.stat()
    except OSError: raise base.Rejected("dedicated_profile_required") from None
    base.need(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & 0o077, "dedicated_profile_required")
    try: marker = json.loads(base.private_read(profile / ".zaturobot-dedicated-browser"))
    except (OSError, ValueError): raise base.Rejected("dedicated_profile_required") from None
    pid = config.value.get("browser_pid")
    base.need(type(pid) is int and pid > 1 and marker == {"schema": 1, "purpose": "zaturobot-browser", "pid": pid, "cdp_endpoint": target}, "dedicated_profile_required")
    try: data = Path(f"/proc/{pid}/cmdline").read_bytes()
    except OSError: raise base.Rejected("browser_identity_changed") from None
    base.need(0 < len(data) <= 32768, "browser_identity_changed")
    args = [v.decode() for v in data.split(b"\0") if v]
    base.need(Path(args[0]).name in {"chrome", "chromium", "chromium-browser", "google-chrome", "headless_shell"}, "dedicated_browser_required")
    def option(name):
        values=[]
        for i, value in enumerate(args):
            if value.startswith(name+"="):values.append(value[len(name)+1:])
            elif value==name and i+1<len(args):values.append(args[i+1])
        return values[0] if len(values)==1 else None
    base.need(option("--user-data-dir") == str(profile) and option("--remote-debugging-port") == str(urllib.parse.urlsplit(target).port) and option("--remote-debugging-address") == "127.0.0.1", "browser_identity_changed")
    switches = {value.split("=", 1)[0] for value in args[1:] if value.startswith("--")}
    base.need(option("--remote-allow-origins") != "*" and not switches & {"--no-sandbox", "--disable-web-security"}, "unsafe_browser_configuration")
    return target


def package_tree_sha256(script):
    """Hash exactly the reviewed three-package npm tree, without importing its code.

    The owner records the digest after an independent tarball/integrity review.
    Canonical JSON maps POSIX paths relative to node_modules to each file's SHA256.
    Generated .bin/.package-lock metadata is not executed or included. Symlinks
    inside any reviewed package, unexpected root packages and oversized trees fail.
    """
    script = Path(script)
    base.need(script.is_absolute() and ".." not in script.parts and len(script.parents) >= 3, "invalid_package_layout")
    root = script.parents[2]
    base.need(root.name == "node_modules" and script.relative_to(root).as_posix() == "@playwright/mcp/cli.js", "invalid_package_layout")
    base.need(not any(p.is_symlink() for p in [root, *root.parents]), "unsafe_package_tree")
    manifest = {}; package_json = {}; total = 0; directories = 0

    def checked_directory(path):
        nonlocal directories
        info = path.lstat(); directories += 1
        base.need(stat.S_ISDIR(info.st_mode) and info.st_uid in (0, os.getuid()) and not info.st_mode & 0o022 and directories <= MAX_TREE_DIRECTORIES, "unsafe_package_tree")

    def children(path, maximum):
        entries = list(itertools.islice(path.iterdir(), maximum + 1))
        base.need(len(entries) <= maximum, "package_tree_too_large")
        return entries

    checked_directory(root)
    entries = children(root, 5)
    base.need(len(entries) <= 5 and {p.name for p in entries} <= {"@playwright", "playwright", "playwright-core", ".bin", ".package-lock.json"}, "unexpected_package_entry")
    scope = root / "@playwright"; checked_directory(scope)
    base.need({p.name for p in children(scope, 1)} == {"mcp"}, "unexpected_package_entry")
    # npm creates these optional metadata entries. Never follow or execute them;
    # .bin's generated links are outside the reviewed package/code trees.
    for item in entries:
        if item.name == ".bin": checked_directory(item)
        elif item.name == ".package-lock.json":
            info = item.lstat()
            base.need(stat.S_ISREG(info.st_mode) and info.st_uid in (0, os.getuid()) and not info.st_mode & 0o022 and info.st_size <= base.MAX_JSON, "unsafe_package_tree")

    for name in sorted(PACKAGES):
        pending = [root / name]
        while pending:
            directory = pending.pop(); checked_directory(directory)
            for path in children(directory, MAX_TREE_FILES + MAX_TREE_DIRECTORIES):
                relative = path.relative_to(root).as_posix()
                base.need(len(relative) <= 512 and len(path.relative_to(root).parts) <= 12 and re.fullmatch(r"[A-Za-z0-9_.@+()-]{1,160}", path.name) is not None, "unsafe_package_tree")
                info = path.lstat()
                base.need(not stat.S_ISLNK(info.st_mode), "unsafe_package_tree")
                if stat.S_ISDIR(info.st_mode):
                    pending.append(path)
                    base.need(len(pending) + directories <= MAX_TREE_DIRECTORIES, "package_tree_too_large")
                    continue
                base.need(stat.S_ISREG(info.st_mode), "unsafe_package_tree")
                base.need(len(manifest) < MAX_TREE_FILES and total + info.st_size <= MAX_TREE_BYTES, "package_tree_too_large")
                fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
                with os.fdopen(fd, "rb") as stream:
                    actual = os.fstat(stream.fileno())
                    base.need(stat.S_ISREG(actual.st_mode) and actual.st_uid in (0, os.getuid()) and not actual.st_mode & 0o022, "unsafe_package_tree")
                    base.need(actual.st_size <= MAX_TREE_FILE_BYTES and total + actual.st_size <= MAX_TREE_BYTES, "package_tree_too_large")
                    data = stream.read(MAX_TREE_FILE_BYTES + 1)
                base.need(len(data) == actual.st_size and len(data) <= MAX_TREE_FILE_BYTES, "package_tree_changed")
                total += len(data); manifest[relative] = hashlib.sha256(data).hexdigest()
                if relative == name + "/package.json":
                    base.need(len(data) <= base.MAX_JSON, "invalid_package_metadata")
                    try: package_json[name] = json.loads(data)
                    except (ValueError, UnicodeError): raise base.Rejected("invalid_package_metadata") from None
    base.need(all(isinstance(package_json.get(name), dict) and package_json[name].get("name") == name and package_json[name].get("version") == version for name, version in PACKAGES.items()), "package_version_mismatch")
    return base.digest(manifest)


def backend_argv(config):
    node = base.checked_artifact(config.value["node"], config.value["node_sha256"])
    script = base.checked_artifact(config.value["playwright_cli"], config.value["playwright_cli_sha256"])
    base.need(node.name in {"node", "nodejs"} and os.access(node, os.X_OK) and script.name == "cli.js", "invalid_executable")
    tree_hash = config.value.get("playwright_tree_sha256")
    base.need(isinstance(tree_hash, str) and re.fullmatch(r"[0-9a-f]{64}", tree_hash), "package_tree_hash_required")
    base.need(package_tree_sha256(script) == tree_hash, "package_tree_hash_mismatch")
    return [str(node), str(script), "--cdp-endpoint", endpoint(config), "--image-responses", "omit", "--snapshot-mode", "none", "--no-webmcp", "--timeout-action", "5000", "--timeout-navigation", "10000"]


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_):return None


def pages(config):
    root = browser_identity(config)
    request = urllib.request.Request(root + "/json/list", headers={"Cache-Control":"no-store"})
    http = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with http.open(request, timeout=min(config.timeout, 3)) as response:
            data = response.read(65537)
            base.need(len(data) <= 65536, "cdp_response_too_large")
            tabs = json.loads(data)
    except urllib.error.HTTPError as error:
        error.close()
        raise base.Rejected("dedicated_browser_unavailable") from None
    except (OSError, ValueError, urllib.error.URLError):
        raise base.Rejected("dedicated_browser_unavailable") from None
    base.need(isinstance(tabs, list) and len(tabs) <= 64, "invalid_cdp_response")
    tabs = [t for t in tabs if isinstance(t, dict) and t.get("type") == "page"]
    base.need(len(tabs) == 1, "single_dedicated_page_required")
    tab = tabs[0]
    base.need(isinstance(tab.get("id"), str) and re.fullmatch(r"[A-Za-z0-9_-]{1,128}", tab["id"]), "invalid_cdp_response")
    url = tab.get("url")
    base.need(isinstance(url, str) and (url == "about:blank" or origin(url) in browser_grant(config)), "origin_not_granted")
    if url != "about:blank": url_credentials_ok(url)
    return {"page_id": tab["id"], "url": url}


def url_credentials_ok(url):
    parsed = urllib.parse.urlsplit(url)
    base.need(not parsed.fragment and not base.SECRET.search(url), "url_credentials_forbidden")
    for key, _ in urllib.parse.parse_qsl(parsed.query):
        base.need(not SENSITIVE.search(key) and key.casefold() not in {"token", "code", "key", "auth", "session", "sid", "password"}, "url_credentials_forbidden")


def public_url(config, url):
    base.need(isinstance(url, str) and 1 <= len(url.encode()) <= 2048 and not any(ord(c) < 33 for c in url), "invalid_url")
    base.need(origin(url) in browser_grant(config), "origin_not_granted")
    parsed = urllib.parse.urlsplit(url)
    url_credentials_ok(url)
    try:
        results = socket.getaddrinfo(parsed.hostname, 443, type=socket.SOCK_STREAM)
    except OSError:
        raise base.Rejected("public_host_unavailable") from None
    base.need(results and all(ipaddress.ip_address(item[4][0]).is_global for item in results), "public_host_required")
    return url


def page_check(config, target):
    base.need(isinstance(target, dict) and set(target) == {"page_id"}, "invalid_target")
    observed = pages(config)
    base.need(observed["page_id"] == target["page_id"], "page_changed")
    return observed


def call_text(session, tool, arguments):
    result = session.rpc("tools/call", {"name":tool,"arguments":arguments})
    base.need(result.get("isError") is not True, "browser_tool_rejected")
    blocks = result.get("content", [])
    base.need(isinstance(blocks, list) and len(blocks) <= 16, "invalid_mcp")
    texts = [b.get("text", "") for b in blocks if isinstance(b, dict) and b.get("type") == "text"]
    base.need(all(isinstance(t, str) for t in texts), "invalid_mcp")
    # Images, resources, recordings, traces and provider error bodies are never forwarded.
    raw = "\n".join(texts)
    base.need(len(raw.encode()) <= 256*1024, "semantic_output_too_large")
    return raw


def snapshot(config, session, target):
    before = page_check(config, target)
    raw = call_text(session, "browser_snapshot", {})
    after = page_check(config, target)
    base.need(before == after, "page_changed")
    lines=[]; protected=set()
    for line in raw.splitlines():
        refs = re.findall(r"\[ref=([A-Za-z0-9_-]+)\]", line)
        if SENSITIVE.search(line) or base.SECRET.search(line):
            protected.update(refs);continue
        lines.append(base.clean_text(line, 512))
    text = "\n".join(lines).encode()[:base.MAX_TEXT].decode("utf-8", errors="ignore")
    result = {"page_id": after["page_id"], "url": after["url"], "text": text, "truncated": len("\n".join(lines).encode()) > base.MAX_TEXT}
    while result["text"] and len(base.encoded(result)) > base.MAX_TEXT:
        excess = len(base.encoded(result)) - base.MAX_TEXT + 128
        result["text"] = result["text"].encode()[:-excess].decode("utf-8", errors="ignore")
        result["truncated"] = True
    base.need(len(base.encoded(result)) <= base.MAX_TEXT, "semantic_output_too_large")
    # JSON escaping/metadata can require a second truncation. Only references in
    # the final emitted observation may authorize an element-bound input.
    refs = set(re.findall(r"\[ref=([A-Za-z0-9_-]+)\]", result["text"]))
    return result, base.digest(result), refs, protected


def input_arguments(request, view, refs, protected):
    tool = request["tool"];args = dict(request["arguments"])
    fields = {"browser_click":{"element","ref"},"browser_type":{"element","ref","text"},"browser_press_key":{"key"}}
    base.need(tool in fields and set(args) == fields[tool], "invalid_arguments")
    if tool == "browser_press_key":
        base.need(args["key"] in base.KEYS, "key_not_allowed")
        # Keyboard input without an explicit element can submit/focus an unknown page control.
        # Only cancellation/navigation keys are safe enough for this compact foundation.
        base.need(args["key"] in {"Escape", "Tab", "Shift+Tab", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "PageUp", "PageDown", "Home", "End"}, "key_not_allowed")
    else:
        base.need(isinstance(args["ref"], str) and REF.fullmatch(args["ref"]) and args["ref"] in refs and args["ref"] not in protected, "stale_element")
        base.need(isinstance(args["element"], str) and 1 <= len(args["element"]) <= 160 and not SENSITIVE.search(args["element"]), "protected_element")
        if tool == "browser_type":
            text = args["text"]
            base.need(isinstance(text, str) and 0 < len(text.encode()) <= 1024 and not any(ord(c) < 32 for c in text) and not base.SECRET.search(text), "invalid_input_text")
            base.need(not re.search(r"javascript:|data:|file:|chrome://|\$\(|`", text, re.I), "unsafe_input_text")
            args["submit"] = False
    return args


def execute(config, request):
    tool = request.get("tool")
    base.need(tool in READ_TOOLS | INPUT_TOOLS, "tool_not_allowed")
    mutating = tool in INPUT_TOOLS
    if mutating: base.input_grant(config, request)
    elif tool == "browser_snapshot": base.need(not request["arguments"], "invalid_arguments")
    else:
        base.need(set(request["arguments"]) == {"url"}, "invalid_arguments")
        public_url(config, request["arguments"]["url"])
    with base.operation(config, request, mutating) as (end, begin):
        page_check(config, request.get("target"))
        with base.Session(config, backend_argv(config), end, expected_version=RUNTIME_VERSION) as session:
            if tool == "browser_navigate":
                begin();call_text(session, tool, request["arguments"])
                current = page_check(config, request["target"])
                public_url(config, current["url"])
            view, fingerprint, refs, protected = snapshot(config, session, request["target"])
            if not mutating:
                return {"schema":1,"status":"completed","tool":tool,"data":view,"snapshot_sha256":fingerprint}
            base.need(request["expected_snapshot_sha256"] == fingerprint, "stale_snapshot")
            args = input_arguments(request, view, refs, protected)
            before = page_check(config, request["target"])
            begin();call_text(session, tool, args)
            after = page_check(config, request["target"])
            # A click may navigate. Its resulting granted public page is observed, not retried.
            base.need(after["url"] == "about:blank" or origin(after["url"]) in browser_grant(config), "origin_not_granted")
            session.check()
            return {"schema":1,"status":"completed","tool":tool,"data":{"dispatched":True,"verified_external_effect":False,"target":request["target"],"navigation_observed":before["url"] != after["url"]}}


def doctor(config):
    page = pages(config)
    with base.Session(config, backend_argv(config), expected_version=RUNTIME_VERSION) as session:
        # An actual snapshot proves the fixed MCP can reach this dedicated page.
        _, _, _, _ = snapshot(config, session, {"page_id":page["page_id"]})
    return {"schema":1,"status":"completed","upstream":UPSTREAM_VERSION,"data":{"dedicated_browser":True,"page":page,"input_granted":config.allow_input,"capture_granted":False,"stop_file":str(config.stop)}}


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--config",type=Path,required=True);parser.add_argument("command",choices=["doctor","action"]);parser.add_argument("--request-json",type=Path)
    args=parser.parse_args();os.umask(0o077)
    try:
        config=base.Config(args.config,backend="browser")
        if args.command=="doctor":base.need(args.request_json is None);result=doctor(config)
        else:
            base.need(args.request_json is not None);result=execute(config,json.loads(base.private_read(args.request_json)))
    except (base.Rejected,OSError,ValueError,KeyError,TypeError) as error:result=base.result_error(error)
    print(json.dumps(result,ensure_ascii=False,separators=(",",":")));return 0 if result["status"]=="completed" else 1


if __name__=="__main__":raise SystemExit(main())
