"""Hermetic restore boundaries: only synthetic files, no accounts, phone or network."""

import errno
import hashlib
import io
import json
import os
from pathlib import Path
import stat
import tarfile
import tempfile
import unittest
import uuid
from unittest.mock import patch
import restore_youtube as restore


def expected(values):
    return {
        name: {
            "path": name,
            "bytes": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        for name, data in values.items()
    }


def archive(values, extra=None):
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w") as output:
        for name, data in values.items():
            member = tarfile.TarInfo("local-data/" + name)
            member.size = len(data)
            output.addfile(member, io.BytesIO(data))
        if extra is not None:
            output.addfile(extra, io.BytesIO(b"") if extra.isreg() else None)
    stream.seek(0)
    return stream


class RestoreContracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.stage = self.root / "stage"
        self.project = self.root / "project"
        self.project.mkdir()
        self.values = {
            ".local/ytbot/topics.txt": b"An exact synthetic topic\n",
            "video/clip.mp4": b"synthetic media bytes",
            "tools/ytbot/account.py": b"# preserved old source overlay\n",
        }

    def test_verified_staging_merges_data_without_overwriting_source_or_owner_changes(
        self,
    ):
        index = expected(self.values)
        restore.stage_tar(archive(self.values), index, self.stage)
        (self.project / "video").mkdir()
        (self.project / "video/clip.mp4").write_bytes(b"owner edited recording")
        (self.project / "tools/ytbot").mkdir(parents=True)
        source = self.project / "tools/ytbot/account.py"
        source.write_text("# improved source\n")
        result = restore.merge_data(self.stage, index, self.project)
        self.assertEqual(result["restored"], 1)
        self.assertEqual(result["conflicts"], ["video/clip.mp4"])
        self.assertEqual(result["quarantined_source"], ["tools/ytbot/account.py"])
        self.assertEqual(source.read_text(), "# improved source\n")
        self.assertEqual(
            (self.project / "video/clip.mp4").read_bytes(), b"owner edited recording"
        )
        self.assertEqual(
            stat.S_IMODE((self.project / ".local/ytbot/topics.txt").stat().st_mode),
            0o600,
        )
        again = restore.merge_data(self.stage, index, self.project)
        self.assertEqual(again["identical"], 1)

    def test_traversal_links_and_duplicate_members_are_refused_before_merge(self):
        for name, kind in [
            ("local-data/../../escape", tarfile.REGTYPE),
            ("local-data/video/link", tarfile.SYMTYPE),
            ("local-data/video/link", tarfile.LNKTYPE),
            ("local-data/video/fifo", tarfile.FIFOTYPE),
        ]:
            with self.subTest(name=name, kind=kind):
                member = tarfile.TarInfo(name)
                member.type = kind
                member.linkname = "../../outside"
                with self.assertRaises((ValueError, tarfile.TarError)):
                    restore.stage_tar(
                        archive({}, member), {}, self.root / str(uuid.uuid4())
                    )
        duplicate = io.BytesIO()
        with tarfile.open(fileobj=duplicate, mode="w") as output:
            for _ in range(2):
                item = tarfile.TarInfo("local-data/video/a")
                item.size = 1
                output.addfile(item, io.BytesIO(b"x"))
        duplicate.seek(0)
        with self.assertRaises(ValueError):
            restore.stage_tar(duplicate, expected({"video/a": b"x"}), self.stage)
        self.assertFalse((self.root / "escape").exists())

    def test_checksum_missing_entry_and_changed_staging_are_refused(self):
        with self.assertRaises(ValueError):
            restore.stage_tar(
                archive({"video/clip.mp4": b"changed"}),
                expected({"video/clip.mp4": b"expected"}),
                self.stage,
            )
        empty = self.root / "missing"
        with self.assertRaises(ValueError):
            restore.stage_tar(archive({}), expected({"video/clip.mp4": b"x"}), empty)
        good = self.root / "good"
        index = expected(self.values)
        restore.stage_tar(archive(self.values), index, good)
        (good / "video/clip.mp4").write_bytes(b"tampered")
        with self.assertRaises(ValueError):
            restore.merge_data(good, index, self.project)

    def test_destination_symlink_never_receives_archive_data(self):
        outside = self.root / "outside"
        outside.mkdir()
        (self.project / "video").symlink_to(outside, target_is_directory=True)
        values = {"video/clip.mp4": b"never outside"}
        index = expected(values)
        restore.stage_tar(archive(values), index, self.stage)
        result = restore.merge_data(self.stage, index, self.project)
        self.assertEqual(result["conflicts"], ["video/clip.mp4"])
        self.assertEqual(list(outside.iterdir()), [])

    def test_existing_browser_profile_is_quarantined_as_a_whole(self):
        values = {
            ".local/bot/profile/Default/Preferences": b'{"old":true}',
            ".local/bot/profile/Default/Cookies": b"old encrypted synthetic cookies",
        }
        index = expected(values)
        restore.stage_tar(archive(values), index, self.stage)
        current = self.project / ".local/bot/profile/Default"
        current.mkdir(parents=True)
        (current / "Preferences").write_text('{"current":true}')
        result = restore.merge_data(self.stage, index, self.project)
        self.assertEqual(len(result["quarantined_browser"]), 2)
        self.assertFalse((current / "Cookies").exists())
        self.assertEqual((current / "Preferences").read_text(), '{"current":true}')

    def test_parts_order_hashes_and_canonical_inbox_paths_are_required(self):
        inbox = self.root / "inbox"
        inbox.mkdir()
        parts = []
        for index, data in enumerate(
            [b"outer encrypted prefix", b"outer encrypted suffix"]
        ):
            identifier = str(uuid.uuid4())
            directory = inbox / identifier
            directory.mkdir()
            file = directory / f"part-{index}"
            file.write_bytes(data)
            parts.append(
                {
                    "index": index,
                    "filename": file.name,
                    "transfer_id": identifier,
                    **restore.checksum(file),
                }
            )
        manifest = {
            "parts": parts,
            "encrypted_archive": {"bytes": sum(x["bytes"] for x in parts)},
        }
        self.assertEqual(len(restore.locate_parts(manifest, inbox)), 2)
        parts[0]["transfer_id"] = "../escape"
        with self.assertRaises(ValueError):
            restore.locate_parts(manifest, inbox)
        parts[0]["transfer_id"] = str(uuid.uuid4())
        parts[0]["index"] = 7
        with self.assertRaises(ValueError):
            restore.locate_parts(manifest, inbox)

    def test_embedded_snapshot_index_must_match_the_authenticated_external_index(self):
        metadata = json.dumps(
            {"schema": 1, "files": list(expected(self.values).values())}
        ).encode()
        digest = {
            "bytes": len(metadata),
            "sha256": hashlib.sha256(metadata).hexdigest(),
            "sha512": hashlib.sha512(metadata).hexdigest(),
        }
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w") as output:
            for name, data in self.values.items():
                member = tarfile.TarInfo("local-data/" + name)
                member.size = len(data)
                output.addfile(member, io.BytesIO(data))
            member = tarfile.TarInfo("SNAPSHOT-INDEX.json")
            member.size = len(metadata)
            output.addfile(member, io.BytesIO(metadata))
        stream.seek(0)
        self.assertEqual(
            restore.stage_tar(stream, expected(self.values), self.stage, digest),
            len(self.values),
        )
        self.assertFalse((self.stage / "SNAPSHOT-INDEX.json").exists())
        stream.seek(0)
        bad = dict(digest)
        bad["sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            restore.stage_tar(
                stream, expected(self.values), self.root / "tampered", bad
            )
        # Accept the producer's detached-file newline without relaxing content hashes.
        detached = self.root / "index.json"
        detached.write_bytes(metadata + b"\n")
        outer = {
            "schema": 1,
            "kind": "youtube-private-migration",
            "format": "age+x25519/zstd/tar",
            "encrypted_archive": {"bytes": 1},
            "compressed_archive": {"bytes": 1},
            "improved_source_revision": "a" * 40,
            "snapshot_index": restore.checksum(detached),
        }
        manifest = self.root / "manifest.json"
        manifest.write_text(json.dumps(outer))
        loaded, entries = restore.load_metadata(manifest, detached)
        stream.seek(0)
        self.assertEqual(
            restore.stage_tar(
                stream, entries, self.root / "newline", loaded["embedded_index_digests"]
            ),
            len(self.values),
        )

    def test_private_staging_rejects_symlinked_local_directory(self):
        outside = self.root / "outside"
        outside.mkdir()
        (self.project / ".local").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(ValueError):
            restore.safe_parent(self.project, ".local/youtube-migrations/placeholder")
        self.assertEqual(list(outside.iterdir()), [])

    def test_cross_filesystem_publication_is_atomic_and_preserves_file_time(self):
        source = self.root / "source"
        source.write_bytes(b"verified synthetic data")
        os.utime(source, (1234567890, 1234567890))
        destination = self.project / "output"
        original = os.link
        calls = []

        def link(old, new, **options):
            calls.append(str(old))
            if len(calls) == 1:
                raise OSError(errno.EXDEV, "synthetic cross-filesystem boundary")
            return original(old, new, **options)

        with patch.object(restore.os, "link", side_effect=link):
            restore.publish(source, destination)
        self.assertEqual(destination.read_bytes(), source.read_bytes())
        self.assertEqual(destination.stat().st_mtime_ns, source.stat().st_mtime_ns)
        with self.assertRaises(FileExistsError):
            restore.publish(source, destination)

    def test_metadata_is_bounded_hashed_and_only_allows_private_data_roots(self):
        index_path = self.root / "index.json"
        index_path.write_text(
            json.dumps({"schema": 1, "files": list(expected(self.values).values())})
        )
        manifest = {
            "schema": 1,
            "kind": "youtube-private-migration",
            "format": "age+x25519/zstd/tar",
            "encrypted_archive": {"bytes": 1},
            "compressed_archive": {"bytes": 1},
            "improved_source_revision": "a" * 40,
            "snapshot_index": restore.checksum(index_path),
        }
        manifest_path = self.root / "manifest.json"
        manifest_path.write_text(json.dumps(manifest))
        _, entries = restore.load_metadata(manifest_path, index_path)
        self.assertEqual(len(entries), 3)
        invalid = {
            "schema": 1,
            "files": list(expected({".git/config": b"never restore"}).values()),
        }
        index_path.write_text(json.dumps(invalid))
        manifest["snapshot_index"] = restore.checksum(index_path)
        manifest_path.write_text(json.dumps(manifest))
        with self.assertRaises(ValueError):
            restore.load_metadata(manifest_path, index_path)


if __name__ == "__main__":
    unittest.main()
