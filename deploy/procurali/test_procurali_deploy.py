#!/usr/bin/env python3
"""Hermetic unit tests for Procurali pull auto-deploy and atomic site symlink swaps."""

from __future__ import annotations

import hashlib
import io
import json
import os
import subprocess
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import package_site_builder
import procurali_deploy_agent as agent
import procurali_site_sync as site_sync


def write_minimal_next_export(site_dir: Path, body: str) -> None:
    for sub in ("ofertas", "api", "_next/static/chunks"):
        (site_dir / sub).mkdir(parents=True, exist_ok=True)
    (site_dir / "index.html").write_text(f"<html>{body}</html>", encoding="utf-8")
    (site_dir / "ofertas" / "index.html").write_text("ok", encoding="utf-8")
    (site_dir / "api" / "catalog.json").write_text("{}", encoding="utf-8")
    (site_dir / "api" / "whatsapp-assisted-drafts.json").write_text("{}", encoding="utf-8")
    (site_dir / "sitemap.xml").write_text("<urlset/>", encoding="utf-8")
    (site_dir / "_next" / "static" / "chunks" / "main.js").write_text("// ok", encoding="utf-8")


class ProcuraliDeployTests(unittest.TestCase):
    def test_verify_ci_checks_requires_all_checks_completed_and_successful(self) -> None:
        ok, _ = agent.verify_ci_checks_passed(
            {
                "check_runs": [
                    {"id": 1, "name": "check", "status": "completed", "conclusion": "success"},
                    {"id": 2, "name": "android", "status": "completed", "conclusion": "success"},
                ]
            }
        )
        self.assertTrue(ok)

        missing_android, msg = agent.verify_ci_checks_passed(
            {
                "check_runs": [
                    {"id": 1, "name": "check", "status": "completed", "conclusion": "success"}
                ]
            }
        )
        self.assertFalse(missing_android)
        self.assertIn("android", msg)

        failed_check, msg = agent.verify_ci_checks_passed(
            {
                "check_runs": [
                    {"id": 10, "name": "check", "status": "completed", "conclusion": "failure"},
                    {"id": 11, "name": "android", "status": "completed", "conclusion": "success"},
                ]
            }
        )
        self.assertFalse(failed_check)
        self.assertIn("failure", msg)

    def test_tar_extraction_rejects_traversal_and_verifies_sha256(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            bad_tar = root / "bad.tar.gz"
            with tarfile.open(bad_tar, "w:gz") as tf:
                info = tarfile.TarInfo("../escape.txt")
                data = b"pwn"
                info.size = len(data)
                tf.addfile(info, io.BytesIO(data))
            with self.assertRaises(ValueError):
                agent.safe_extract_tar_gz(bad_tar, root / "out")

            good_tar = root / "good.tar.gz"
            hub_bytes = b"#!/bin/sh\necho ok\n"
            with tarfile.open(good_tar, "w:gz") as tf:
                info = tarfile.TarInfo("bin/zaturo-hub")
                info.size = len(hub_bytes)
                info.mode = 0o755
                tf.addfile(info, io.BytesIO(hub_bytes))
            out_dir = root / "release"
            agent.safe_extract_tar_gz(good_tar, out_dir)
            manifest = {
                "files": {
                    "bin/zaturo-hub": hashlib.sha256(hub_bytes).hexdigest(),
                }
            }
            agent.verify_manifest_and_binaries(out_dir, manifest)
            with self.assertRaises(ValueError):
                agent.verify_manifest_and_binaries(
                    out_dir, {"files": {"bin/zaturo-hub": "0" * 64}}
                )

    def test_blue_green_swap_cuts_caddy_to_standby_and_rolls_back_on_standby_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            opt = root / "opt"
            state = root / "state"
            rel = opt / "staging" / "abc123"
            (rel / "bin").mkdir(parents=True, exist_ok=True)
            (rel / "bin" / "zaturo-hub").write_bytes(b"hub-v2")
            (rel / "bin" / "zaturo-worker").write_bytes(b"worker-v2")
            (opt / "bin").mkdir(parents=True, exist_ok=True)

            paths = agent.DeployPaths(
                opt_root=opt,
                state_dir=state,
                hub_db=root / "nonexistent.db",
                hub_backups_dir=root / "backups",
                caddy_main_config=root / "Caddyfile",
                caddy_upstream_file=root / "procurali-upstream.caddy",
                site_live_link=root / "site",
            )
            paths.active_slot_file.write_text("a\n", encoding="utf-8")
            manifest = {"commit_sha": "abc123def4567890", "release_tag": "procurali-abc123def456"}

            recorded_cmds: list[list[str]] = []

            def fake_cmd(args: list[str], *, check: bool = True, timeout: int = 60):
                recorded_cmds.append(args)
                return subprocess.CompletedProcess(args, 0, "", "")

            # Failure during standby health check must keep slot `a` active and stop `b`.
            def fail_standby(_port: int) -> None:
                raise RuntimeError("boom")

            with self.assertRaises(RuntimeError):
                agent.deploy_blue_green_release(
                    paths,
                    manifest,
                    rel,
                    health_check_fn=fail_standby,
                    cmd_runner=fake_cmd,
                    drain_seconds=0,
                )
            self.assertEqual(agent.read_active_slot(paths.active_slot_file), "a")
            self.assertFalse(paths.caddy_upstream_file.exists())
            self.assertIn(["systemctl", "stop", "procurali-hub@b"], recorded_cmds)

            # Successful standby cutover switches active slot a -> b cleanly.
            recorded_cmds.clear()
            checked_ports: list[int] = []

            def ok_health(port: int) -> None:
                checked_ports.append(port)

            rec = agent.deploy_blue_green_release(
                paths,
                manifest,
                rel,
                health_check_fn=ok_health,
                cmd_runner=fake_cmd,
                drain_seconds=0,
            )
            self.assertEqual(rec["active_slot"], "b")
            self.assertEqual(checked_ports, [17882, 7878])
            self.assertIn("127.0.0.1:17882", paths.caddy_upstream_file.read_text(encoding="utf-8"))
            self.assertEqual(agent.read_active_slot(paths.active_slot_file), "b")
            self.assertEqual(paths.current_link.resolve(), rel.resolve())
            self.assertIn(["systemctl", "reload", "caddy"], recorded_cmds)
            self.assertIn(["systemctl", "stop", "procurali-hub@a"], recorded_cmds)

    def test_atomic_symlink_swap_preserves_live_site_without_gap(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            rel1 = root / "releases" / "r1"
            rel2 = root / "releases" / "r2"
            for r, body in ((rel1, "v1"), (rel2, "v2")):
                write_minimal_next_export(r, body)
                site_sync.verify_built_site(r)

            live = root / "site"
            site_sync.atomic_symlink_swap(rel1, live)
            self.assertEqual((live / "index.html").read_text(encoding="utf-8"), "<html>v1</html>")
            site_sync.atomic_symlink_swap(rel2, live)
            self.assertEqual((live / "index.html").read_text(encoding="utf-8"), "<html>v2</html>")
            self.assertEqual(os.readlink(live), str(rel2))

    def test_verify_built_site_requires_next_export_assets(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            site = Path(tmp) / "site"
            write_minimal_next_export(site, "v1")
            site_sync.verify_built_site(site)
            (site / "_next" / "static" / "chunks" / "main.js").unlink()
            (site / "_next" / "static" / "chunks").rmdir()
            with self.assertRaises(RuntimeError):
                site_sync.verify_built_site(site)

    def test_builder_digest_tracks_sources_but_ignores_generated_output(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "site-builder"
            (root / "src").mkdir(parents=True)
            (root / "app").mkdir()
            builder = root / "src" / "build.mjs"
            builder.write_text("// builder", encoding="utf-8")
            (root / "app" / "page.tsx").write_text("export default 1;", encoding="utf-8")
            (root / "package-lock.json").write_text("{}", encoding="utf-8")
            link = Path(tmp) / "current-build.mjs"
            link.symlink_to(builder)
            first = site_sync.compute_builder_digest(link)
            self.assertEqual(first, site_sync.compute_builder_digest(builder))

            for generated in (".generated", ".next", "public/api", "out-sitebuild-1", "node_modules/x"):
                (root / generated).mkdir(parents=True, exist_ok=True)
                (root / generated / "file.json").write_text("{}", encoding="utf-8")
            (root / "next-env.d.ts").write_text("// generated", encoding="utf-8")
            self.assertEqual(first, site_sync.compute_builder_digest(builder))

            (root / "app" / "page.tsx").write_text("export default 2;", encoding="utf-8")
            self.assertNotEqual(first, site_sync.compute_builder_digest(builder))

    def test_node_executable_prefers_env_then_bundled_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "site-builder"
            builder = root / "src" / "build.mjs"
            builder.parent.mkdir(parents=True)
            builder.write_text("// builder", encoding="utf-8")
            saved = os.environ.pop("PROCURALI_SITE_NODE", None)
            try:
                self.assertEqual(site_sync.resolve_node_executable(builder), "node")
                runtime = root / "runtime" / "node"
                runtime.parent.mkdir()
                runtime.write_text("#!/bin/sh\n", encoding="utf-8")
                runtime.chmod(0o755)
                self.assertEqual(site_sync.resolve_node_executable(builder), str(runtime))
                os.environ["PROCURALI_SITE_NODE"] = "/usr/local/bin/node"
                self.assertEqual(site_sync.resolve_node_executable(builder), "/usr/local/bin/node")
            finally:
                os.environ.pop("PROCURALI_SITE_NODE", None)
                if saved is not None:
                    os.environ["PROCURALI_SITE_NODE"] = saved

    def test_failed_builds_back_off_up_to_fifteen_minutes(self) -> None:
        self.assertEqual(site_sync.failure_backoff_secs(0, poll_secs=5), 5)
        self.assertEqual(site_sync.failure_backoff_secs(1, poll_secs=5), 30)
        self.assertEqual(site_sync.failure_backoff_secs(3, poll_secs=5), 120)
        self.assertEqual(site_sync.failure_backoff_secs(20, poll_secs=5), 900)

    def test_package_site_builder_ships_only_regular_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "prototype"
            for rel in package_site_builder.REQUIRED_PATHS:
                (source / rel).parent.mkdir(parents=True, exist_ok=True)
                (source / rel).write_text("// fixture", encoding="utf-8")
            for generated in (".next/cache", "out", "public/api", ".generated", "test", "out-x"):
                (source / generated).mkdir(parents=True, exist_ok=True)
            bin_dir = source / "node_modules" / ".bin"
            bin_dir.mkdir(parents=True)
            (bin_dir / "next").symlink_to("../next/dist/bin/next")
            for pruned in ("@next/swc-linux-x64-musl", "@img/sharp-linux-x64", "lucide-react"):
                (source / "node_modules" / pruned).mkdir(parents=True)
                (source / "node_modules" / pruned / "index.js").write_text("//", encoding="utf-8")
            node = Path(tmp) / "node"
            node.write_text("#!/bin/sh\n", encoding="utf-8")

            dest = Path(tmp) / "bundle" / "site-builder"
            removed = package_site_builder.package_site_builder(source, dest, node)
            self.assertIn(".bin", removed)
            self.assertIn("@next/swc-linux-x64-musl", removed)
            self.assertTrue((dest / "node_modules" / "lucide-react" / "index.js").is_file())
            self.assertTrue((dest / "src" / "build.mjs").is_file())
            self.assertTrue(os.access(dest / "runtime" / "node", os.X_OK))
            for generated in (".next", "out", "public", ".generated", "test", "out-x"):
                self.assertFalse((dest / generated).exists(), generated)

            archive = Path(tmp) / "bundle.tar.gz"
            with tarfile.open(archive, "w:gz") as tf:
                tf.add(dest.parent, arcname=".")
            agent.safe_extract_tar_gz(archive, Path(tmp) / "extracted")

            stray = source / "node_modules" / "lucide-react" / "link.js"
            stray.symlink_to("index.js")
            with self.assertRaises(ValueError):
                package_site_builder.package_site_builder(source, Path(tmp) / "again", None)

    def test_failed_rebuild_site_cleans_up_release_dir_and_keeps_live_site(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            live_target = root / "releases" / "bootstrap"
            write_minimal_next_export(live_target, "bootstrap")

            live_link = root / "site"
            site_sync.atomic_symlink_swap(live_target, live_link)

            broken_builder = root / "builder" / "src" / "build.mjs"
            broken_builder.parent.mkdir(parents=True, exist_ok=True)
            broken_builder.write_text("console.error('boom'); process.exit(1);\n", encoding="utf-8")
            symlink_builder = root / "current-build.mjs"
            symlink_builder.symlink_to(broken_builder)

            releases_dir = root / "releases"
            state_dir = root / "state"
            with self.assertRaises(subprocess.CalledProcessError) as ctx:
                site_sync.rebuild_site(
                    b'{"offers":[]}',
                    "etag-1",
                    "digest-1",
                    builder_script=symlink_builder,
                    releases_dir=releases_dir,
                    live_site_link=live_link,
                    state_dir=state_dir,
                    chown_caddy=False,
                )
            self.assertIn("boom", ctx.exception.stderr)
            self.assertEqual(os.readlink(live_link), str(live_target))
            self.assertEqual((live_link / "index.html").read_text(encoding="utf-8"), "<html>bootstrap</html>")
            self.assertEqual([p.name for p in releases_dir.iterdir()], ["bootstrap"])

    def test_idle_run_once_short_circuits_without_rest_api_calls(self) -> None:
        self.assertEqual(
            agent.parse_procurali_tag_from_url(
                "https://github.com/AlexandreZanata/ZaturoBot/releases/tag/procurali-05493d3cfd1c"
            ),
            "procurali-05493d3cfd1c",
        )
        self.assertIsNone(
            agent.parse_procurali_tag_from_url(
                "https://github.com/AlexandreZanata/ZaturoBot/releases/tag/v0.9.0"
            )
        )
        self.assertEqual(
            agent.parse_latest_procurali_tag_from_atom(
                '<entry><link href="/AlexandreZanata/ZaturoBot/releases/tag/v1.0"/></entry>'
                '<entry><link href="/AlexandreZanata/ZaturoBot/releases/tag/procurali-abcdef123456"/></entry>'
            ),
            "procurali-abcdef123456",
        )

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            paths = agent.DeployPaths(opt_root=root / "opt", state_dir=root / "state")
            agent.write_deploy_record(
                paths.deploy_record_file,
                {
                    "status": "ok",
                    "commit_sha": "abcdef1234567890",
                    "release_tag": "procurali-abcdef123456",
                },
            )

            def fail_rest(*_args, **_kwargs):
                raise AssertionError("idle poll must not consume GitHub REST API quota")

            out = agent.run_once(
                repo="AlexandreZanata/ZaturoBot",
                paths=paths,
                required_checks=("check", "android"),
                latest_tag_fn=lambda _repo: "procurali-abcdef123456",
                get_json_fn=fail_rest,
                download_fn=fail_rest,
            )
            self.assertEqual(out["status"], "up_to_date")
            self.assertEqual(out["release_tag"], "procurali-abcdef123456")

    def test_blue_green_swap_updates_installed_deploy_agent_when_present(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            opt = root / "opt"
            (opt / "agent").mkdir(parents=True, exist_ok=True)
            rel = opt / "releases" / "deadbeef"
            (rel / "bin").mkdir(parents=True, exist_ok=True)
            (rel / "bin" / "zaturo-hub").write_bytes(b"hub")
            (rel / "bin" / "zaturo-worker").write_bytes(b"worker")
            (rel / "deploy" / "procurali").mkdir(parents=True, exist_ok=True)
            (rel / "deploy" / "procurali" / "procurali_deploy_agent.py").write_text(
                "# agent-v2\n", encoding="utf-8"
            )

            paths = agent.DeployPaths(
                opt_root=opt,
                state_dir=root / "state",
                hub_db=root / "nonexistent.db",
                hub_backups_dir=root / "backups",
                caddy_main_config=root / "Caddyfile",
                caddy_upstream_file=root / "procurali-upstream.caddy",
                site_live_link=root / "site",
            )
            paths.active_slot_file.write_text("b\n", encoding="utf-8")

            def fake_cmd(args: list[str], *, check: bool = True, timeout: int = 60):
                return subprocess.CompletedProcess(args, 0, "", "")

            rec = agent.deploy_blue_green_release(
                paths,
                {"commit_sha": "deadbeef12345678", "release_tag": "procurali-deadbeef1234"},
                rel,
                health_check_fn=lambda _port: None,
                cmd_runner=fake_cmd,
                drain_seconds=0,
            )
            self.assertEqual(rec["active_slot"], "a")
            self.assertEqual(
                (opt / "agent" / "procurali-deploy-agent.py").read_text(encoding="utf-8"),
                "# agent-v2\n",
            )

    def test_find_latest_procurali_release_selects_newest_published_at(self) -> None:
        assets = [
            {"name": agent.BUNDLE_ASSET_NAME},
            {"name": agent.MANIFEST_ASSET_NAME},
        ]
        # GitHub's /releases list is not guaranteed to be sorted by published_at descending.
        releases = [
            {
                "id": 100,
                "tag_name": "procurali-oldfirst",
                "published_at": "2026-11-16T18:39:26Z",
                "assets": assets,
            },
            {
                "id": 102,
                "tag_name": "procurali-newest",
                "published_at": "2026-11-16T19:28:48Z",
                "assets": assets,
            },
            {
                "id": 101,
                "tag_name": "procurali-middle",
                "published_at": "2026-11-16T19:05:11Z",
                "assets": assets,
            },
        ]
        selected = agent.find_latest_procurali_release(releases)
        self.assertIsNotNone(selected)
        self.assertEqual(selected["tag_name"], "procurali-newest")


if __name__ == "__main__":
    unittest.main()
