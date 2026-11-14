#!/usr/bin/env python3
"""Verify/decrypt an owner-issued YoutubeBot snapshot, stage it, then merge private data safely."""

from __future__ import annotations
import argparse
import errno
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import subprocess
import tarfile
import tempfile
import uuid

MAX_ARCHIVE = 16 * 1024**3
MAX_UNPACKED = 32 * 1024**3
MAX_FILES = 100_000
DATA_PREFIXES = [
    (".local", "bot"),
    (".local", "ytbot"),
    (".local", "tasks"),
    (".local", "phases"),
    ("imagens",),
    ("audio",),
    ("background-music",),
    ("video",),
]
OVERLAYS = {"tools/ytbot/account.py", "tools/ytbot/THUMBNAIL-RECOVERY.md"}
BROWSER_PREFIXES = [(".local", "bot", "profile"), (".local", "ytbot", "profile")]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def safe_relative(value):
    require(
        isinstance(value, str) and value and len(value.encode()) <= 4096,
        "invalid relative path",
    )
    require(
        "\\" not in value
        and not value.startswith("/")
        and all(p not in ("", ".", "..") for p in value.split("/")),
        "unsafe relative path",
    )
    require(not any(ord(c) < 32 for c in value), "control characters in path")
    path = PurePosixPath(value)
    require(str(path) == value, "noncanonical relative path")
    return path


def data_path(value):
    parts = safe_relative(value).parts
    return value == ".local/README.md" or any(
        parts[: len(prefix)] == prefix and len(parts) > len(prefix)
        for prefix in DATA_PREFIXES
    )


def private_dir(path):
    path.mkdir(parents=True, mode=0o700, exist_ok=True)
    info = path.lstat()
    require(
        stat.S_ISDIR(info.st_mode) and not path.is_symlink(),
        "a real private directory is required",
    )
    require(info.st_uid == os.getuid(), "private directory belongs to another user")
    path.chmod(0o700)
    return path


def real_file(path, limit=MAX_ARCHIVE):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    stream = os.fdopen(descriptor, "rb")
    info = os.fstat(stream.fileno())
    if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
        stream.close()
        raise ValueError("a bounded regular file is required")
    return stream


def checksum(path):
    sha256, sha512 = hashlib.sha256(), hashlib.sha512()
    size = 0
    with real_file(path) as stream:
        while chunk := stream.read(1024 * 1024):
            size += len(chunk)
            sha256.update(chunk)
            sha512.update(chunk)
    return {"bytes": size, "sha256": sha256.hexdigest(), "sha512": sha512.hexdigest()}


def verify_digest(path, expected):
    actual = checksum(path)
    require(
        all(actual[key] == expected[key] for key in ("bytes", "sha256", "sha512")),
        "file size or checksum mismatch",
    )


def read_json(path, limit):
    with real_file(path, limit) as stream:
        return json.load(stream)


def load_metadata(manifest_path, index_path):
    manifest = read_json(manifest_path, 1024 * 1024)
    require(
        manifest.get("schema") == 1
        and manifest.get("kind") == "youtube-private-migration",
        "unsupported migration manifest",
    )
    require(
        manifest.get("format") == "age+x25519/zstd/tar", "unsupported archive format"
    )
    require(
        0 < manifest["encrypted_archive"]["bytes"] <= MAX_ARCHIVE,
        "encrypted archive exceeds limit",
    )
    require(
        0 < manifest["compressed_archive"]["bytes"] <= MAX_ARCHIVE,
        "compressed archive exceeds limit",
    )
    require(
        re.fullmatch(r"[0-9a-f]{40}", manifest.get("improved_source_revision", ""))
        is not None,
        "reviewed source revision is required",
    )
    with real_file(index_path, 32 * 1024 * 1024):
        pass
    verify_digest(index_path, manifest["snapshot_index"])
    # The snapshot producer adds one newline to the detached JSON file only.
    # Both accepted byte representations are derived from the authenticated file.
    index_bytes = index_path.read_bytes()
    manifest["embedded_index_digests"] = [manifest["snapshot_index"]]
    if index_bytes.endswith(b"\n"):
        compact = index_bytes[:-1]
        manifest["embedded_index_digests"].append(
            {
                "bytes": len(compact),
                "sha256": hashlib.sha256(compact).hexdigest(),
                "sha512": hashlib.sha512(compact).hexdigest(),
            }
        )
    index = read_json(index_path, 32 * 1024 * 1024)
    require(
        index.get("schema") == 1
        and isinstance(index.get("files"), list)
        and 0 < len(index["files"]) <= MAX_FILES,
        "invalid snapshot index",
    )
    files = {}
    total = 0
    for item in index["files"]:
        name = str(safe_relative(item["path"]))
        require(
            data_path(name) or name in OVERLAYS,
            "snapshot path is outside the approved data roots",
        )
        require(
            name not in files
            and type(item["bytes"]) is int
            and 0 <= item["bytes"] <= MAX_UNPACKED,
            "duplicate or invalid snapshot entry",
        )
        require(
            re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is not None,
            "invalid snapshot checksum",
        )
        files[name] = item
        total += item["bytes"]
    require(total <= MAX_UNPACKED, "unpacked snapshot exceeds limit")
    manifest["unpacked_bytes"] = total
    return manifest, files


def verify_project(project, revision):
    require(
        project.is_dir() and not project.is_symlink(),
        "choose an existing real YoutubeBot checkout",
    )
    actual = subprocess.check_output(
        ["git", "-C", str(project), "rev-parse", "HEAD"],
        text=True,
        stderr=subprocess.DEVNULL,
    ).strip()
    require(
        actual == revision,
        "check out the reviewed YoutubeBot source revision before restoring private data",
    )


def locate_parts(manifest, inbox):
    require(1 <= len(manifest["parts"]) <= 32, "invalid part count")
    parts = []
    total = 0
    for position, item in enumerate(manifest["parts"]):
        require(item["index"] == position, "parts are not in canonical order")
        require(
            str(uuid.UUID(item["transfer_id"])) == item["transfer_id"],
            "invalid transfer ID",
        )
        name = safe_relative(item["filename"])
        require(
            len(name.parts) == 1 and 0 < item["bytes"] <= 1024**3,
            "invalid part filename or size",
        )
        folder = inbox / item["transfer_id"]
        require(
            folder.is_dir() and not folder.is_symlink(),
            "missing or unsafe inbox transfer folder",
        )
        path = folder / str(name)
        verify_digest(path, item)
        parts.append(path)
        total += item["bytes"]
    require(
        total == manifest["encrypted_archive"]["bytes"],
        "parts do not match encrypted archive length",
    )
    return parts


def decrypt_parts(parts, manifest, identity, destination):
    require(
        stat.S_IMODE(identity.lstat().st_mode) & 0o077 == 0
        and not identity.is_symlink(),
        "age identity must be a private regular file",
    )
    with real_file(identity, 4096):
        pass
    age = shutil.which("age")
    require(age is not None, "install the standard age CLI first")
    sha256, sha512 = hashlib.sha256(), hashlib.sha512()
    total = 0
    with destination.open("xb") as output:
        os.fchmod(output.fileno(), 0o600)
        process = subprocess.Popen(
            [age, "--decrypt", "--identity", str(identity)],
            stdin=subprocess.PIPE,
            stdout=output,
            stderr=subprocess.DEVNULL,
        )
        try:
            for path in parts:
                with real_file(path) as stream:
                    while chunk := stream.read(1024 * 1024):
                        sha256.update(chunk)
                        sha512.update(chunk)
                        total += len(chunk)
                        process.stdin.write(chunk)
            process.stdin.close()
            require(
                process.wait(timeout=600) == 0,
                "age decryption or authentication failed",
            )
            output.flush()
            os.fsync(output.fileno())
        except BaseException:
            process.kill()
            process.wait()
            raise
    require(
        {"bytes": total, "sha256": sha256.hexdigest(), "sha512": sha512.hexdigest()}
        == manifest["encrypted_archive"],
        "assembled encrypted archive checksum mismatch",
    )
    verify_digest(destination, manifest["compressed_archive"])


def stage_tar(stream, expected, destination, embedded_index_digest=None):
    private_dir(destination)
    seen = set()
    embedded_seen = False
    with tarfile.open(fileobj=stream, mode="r|") as archive:
        for member in archive:
            if member.name == "SNAPSHOT-INDEX.json":
                require(
                    not embedded_seen
                    and embedded_index_digest is not None
                    and member.isreg()
                    and member.sparse is None,
                    "unexpected embedded snapshot index",
                )
                candidates = (
                    embedded_index_digest
                    if isinstance(embedded_index_digest, list)
                    else [embedded_index_digest]
                )
                require(
                    any(member.size == item["bytes"] for item in candidates)
                    and member.size <= 32 * 1024 * 1024,
                    "embedded index exceeds its declared limit",
                )
                sha256, sha512 = hashlib.sha256(), hashlib.sha512()
                count = 0
                with archive.extractfile(member) as metadata:
                    while chunk := metadata.read(1024 * 1024):
                        count += len(chunk)
                        sha256.update(chunk)
                        sha512.update(chunk)
                require(
                    count == member.size
                    and any(
                        count == item["bytes"]
                        and sha256.hexdigest() == item["sha256"]
                        and sha512.hexdigest() == item["sha512"]
                        for item in candidates
                    ),
                    "embedded snapshot index checksum mismatch",
                )
                embedded_seen = True
                continue
            require(member.name.startswith("local-data/"), "unexpected archive wrapper")
            name = member.name[len("local-data/") :]
            if member.isdir():
                value = str(safe_relative(name.rstrip("/")))
                require(
                    any(
                        str(PurePosixPath(*prefix)).startswith(value + "/")
                        or value == str(PurePosixPath(*prefix))
                        or value.startswith(str(PurePosixPath(*prefix)) + "/")
                        for prefix in DATA_PREFIXES
                    ),
                    "unapproved archive directory",
                )
                continue
            safe_relative(name)
            require(
                member.isreg()
                and member.sparse is None
                and not member.pax_headers.get("GNU.sparse.map"),
                "archive links, special and sparse files are refused",
            )
            require(
                name in expected
                and name not in seen
                and member.size == expected[name]["bytes"],
                "archive/index entry mismatch",
            )
            path = destination / name
            private_dir(path.parent)
            descriptor = os.open(
                path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600
            )
            digest = hashlib.sha256()
            count = 0
            with (
                os.fdopen(descriptor, "wb") as output,
                archive.extractfile(member) as source,
            ):
                while chunk := source.read(1024 * 1024):
                    count += len(chunk)
                    require(count <= member.size, "archive entry exceeds declared size")
                    digest.update(chunk)
                    output.write(chunk)
                output.flush()
                os.fsync(output.fileno())
            require(
                count == member.size and digest.hexdigest() == expected[name]["sha256"],
                "staged data checksum mismatch",
            )
            os.utime(path, (member.mtime, member.mtime), follow_symlinks=False)
            seen.add(name)
    require(seen == set(expected), "archive does not contain every indexed file")
    return len(seen)


def unpack(compressed, expected, destination, embedded_index_digest=None):
    zstd = shutil.which("zstd")
    require(zstd is not None, "install zstd first")
    process = subprocess.Popen(
        [zstd, "--decompress", "--stdout", str(compressed)],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    try:
        count = stage_tar(process.stdout, expected, destination, embedded_index_digest)
        # Drain only tar padding. A decompression bomb after the archive is rejected.
        trailing = 0
        while chunk := process.stdout.read(1024 * 1024):
            trailing += len(chunk)
            require(
                trailing <= 1024 * 1024 and not any(chunk),
                "unexpected bytes after tar archive",
            )
        require(process.wait(timeout=30) == 0, "zstd decompression failed")
        return count
    except BaseException:
        process.kill()
        process.wait()
        raise
    finally:
        process.stdout.close()


def safe_parent(root, relative):
    current = root
    for component in safe_relative(relative).parts[:-1]:
        current = current / component
        require(not current.is_symlink(), "destination parent is a symlink")
        private_dir(current)
    return current


def publish(source, destination):
    try:
        os.link(source, destination, follow_symlinks=False)
    except OSError as error:
        if error.errno != errno.EXDEV:
            raise
        descriptor, name = tempfile.mkstemp(
            prefix=".zaturo-restore-", dir=destination.parent
        )
        temporary = Path(name)
        try:
            with os.fdopen(descriptor, "wb") as output, real_file(source) as incoming:
                shutil.copyfileobj(incoming, output, 1024 * 1024)
                output.flush()
                os.fsync(output.fileno())
            metadata = source.stat()
            os.utime(
                temporary,
                ns=(metadata.st_atime_ns, metadata.st_mtime_ns),
                follow_symlinks=False,
            )
            os.link(temporary, destination, follow_symlinks=False)
        finally:
            temporary.unlink(missing_ok=True)
    descriptor = os.open(destination.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def merge_data(staged, expected, project):
    result = {
        "restored": 0,
        "identical": 0,
        "conflicts": [],
        "quarantined_source": [],
        "quarantined_browser": [],
    }
    occupied = []
    for prefix in BROWSER_PREFIXES:
        candidate = project.joinpath(*prefix)
        if candidate.is_symlink() or (
            candidate.exists() and (not candidate.is_dir() or any(candidate.iterdir()))
        ):
            occupied.append(prefix)
    for name in sorted(expected):
        source = staged / name
        require(
            checksum(source)["sha256"] == expected[name]["sha256"],
            "staged content changed before merge",
        )
        if not data_path(name):
            result["quarantined_source"].append(name)
            continue
        parts = PurePosixPath(name).parts
        if any(parts[: len(prefix)] == prefix for prefix in occupied):
            result["quarantined_browser"].append(name)
            continue
        destination = project / name
        try:
            safe_parent(project, name)
        except (ValueError, FileExistsError, NotADirectoryError):
            result["conflicts"].append(name)
            continue
        if destination.exists() or destination.is_symlink():
            if (
                not destination.is_symlink()
                and destination.is_file()
                and destination.stat().st_size == expected[name]["bytes"]
                and checksum(destination)["sha256"] == expected[name]["sha256"]
            ):
                result["identical"] += 1
            else:
                result["conflicts"].append(name)
            continue
        try:
            publish(source, destination)
            result["restored"] += 1
        except FileExistsError:
            result["conflicts"].append(name)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--index", type=Path, required=True)
    parser.add_argument(
        "--inbox", type=Path, default=Path.home() / ".local/share/zaturobot/inbox"
    )
    parser.add_argument(
        "--identity",
        type=Path,
        default=Path.home() / ".config/zaturobot/transfer.identity",
    )
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--apply-data", action="store_true")
    args = parser.parse_args()
    os.umask(0o077)
    manifest, expected = load_metadata(args.manifest, args.index)
    project = args.project.absolute()
    verify_project(project, manifest["improved_source_revision"])
    parts = locate_parts(manifest, args.inbox.absolute())
    safe_parent(project, ".local/youtube-migrations/placeholder")
    stage_parent = project / ".local/youtube-migrations"
    private_dir(stage_parent)
    require(
        shutil.disk_usage(stage_parent).free
        >= manifest["compressed_archive"]["bytes"]
        + manifest["unpacked_bytes"]
        + 512 * 1024**2,
        "insufficient free disk space for verified private staging",
    )
    stage = private_dir(stage_parent / str(uuid.uuid4()))
    compressed = stage / "snapshot.tar.zst"
    print(
        "Verifying encrypted parts and staging private data; incoming content is never executed.",
        flush=True,
    )
    try:
        decrypt_parts(parts, manifest, args.identity, compressed)
        unpack(compressed, expected, stage / "data", manifest["embedded_index_digests"])
        result = (
            merge_data(stage / "data", expected, project)
            if args.apply_data
            else {"staged_only": True}
        )
        result.update(
            schema=1,
            source_revision=manifest["improved_source_revision"],
            indexed_files=len(expected),
            staged_directory=str(stage),
        )
        report = stage / "restore-report.json"
        report.write_text(json.dumps(result, indent=2) + "\n")
        report.chmod(0o600)
        print(
            json.dumps(
                {
                    "indexed_files": len(expected),
                    "restored": result.get("restored", 0),
                    "identical": result.get("identical", 0),
                    "conflicts": len(result.get("conflicts", [])),
                    "quarantined_source": len(result.get("quarantined_source", [])),
                    "quarantined_browser": len(result.get("quarantined_browser", [])),
                    "report": str(report),
                },
                indent=2,
            )
        )
    finally:
        # Remove only this invocation's temporary decrypted container; source parts and staged data remain.
        compressed.unlink(missing_ok=True)


if __name__ == "__main__":
    try:
        main()
    except (
        ValueError,
        OSError,
        subprocess.SubprocessError,
        tarfile.TarError,
        KeyError,
        TypeError,
    ):
        raise SystemExit(
            "Restore failed safely. Check the reviewed manifest, available space and private report/staging; no credential values were logged."
        )
