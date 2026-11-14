"""Hermetic updater acceptance: no network, production hub, account, browser or phone."""

import contextlib
import importlib.util
import io
import json
import sqlite3
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import tomllib

spec = importlib.util.spec_from_file_location(
    "work_updater", Path(__file__).with_name("install_work_services.py")
)
updater = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updater)


def write(path, data, mode=0o600):
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    path.write_bytes(data.encode() if isinstance(data, str) else data)
    path.chmod(mode)
    return path


def git(root, *arguments):
    return (
        subprocess.run(
            ["git", "-C", str(root), *arguments], check=True, capture_output=True
        )
        .stdout.decode()
        .strip()
    )


def commit(root):
    git(root, "init", "-q")
    git(root, "add", ".")
    git(
        root,
        "-c",
        "user.name=Synthetic test",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-qm",
        "fixture",
    )
    return git(root, "rev-parse", "HEAD")


class WorkServicesTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="zaturo-updater-test-")
        self.base = Path(self.temporary.name)
        self.home = self.base / "home"
        self.home.mkdir(mode=0o700)
        self.source = self.base / "ZaturoBot"
        self.source.mkdir()
        self.youtube = self.base / "YoutubeBot"
        self.youtube.mkdir()
        write(self.source / ".gitignore", "target/\n")
        write(
            self.source / "assets/brand/zaturobot-icon.svg",
            '<svg xmlns="http://www.w3.org/2000/svg"/>',
        )
        write(
            self.source / "apps/zaturo-desktop/packaging/zaturobot.desktop",
            "[Desktop Entry]\nType=Application\nExec=zaturo-desktop\nIcon=zaturobot\n",
        )
        write(
            self.source / "deploy/notebook/zaturo-worker.user.service",
            "[Service]\nExecStart=%h/.local/lib/zaturobot/zaturo-worker --config %h/.config/zaturobot/worker.toml connect\nMemoryMax=512M\n",
        )
        write(self.youtube / ".gitignore", ".venv/\n.local/\ndist/\n")
        write(
            self.youtube / "tools/ytbot/service.py",
            "# reviewed synthetic fixture; never executed\n",
        )
        write(
            self.youtube / "scripts/build-chrome.py",
            'from pathlib import Path\np = Path("dist/chrome"); p.mkdir(parents=True, exist_ok=True)\n(p / "manifest.json").write_text("{}")\n',
        )
        source_revision, youtube_revision = commit(self.source), commit(self.youtube)
        venv = self.youtube / ".venv/bin"
        venv.mkdir(parents=True)
        (venv / "python").symlink_to(sys.executable)
        self.config_dir = self.home / ".config/zaturobot"
        self.config_dir.mkdir(parents=True, mode=0o700)
        self.env = write(
            self.config_dir / "worker.env",
            "ZATURO_WORKER_TOKEN=synthetic-worker-secret-preserved\n",
        )
        self.identity = write(
            self.config_dir / "transfer.identity", "synthetic-age-secret-preserved\n"
        )
        self.ssh = write(
            self.home / ".ssh/existing.key", "synthetic-ssh-secret-preserved\n"
        )
        self.models = write(
            self.config_dir / "models.toml", "# existing private selection preserved\n"
        )
        self.database = self.home / ".local/share/zaturobot/worker.db"
        self.database.parent.mkdir(parents=True, mode=0o700)
        with sqlite3.connect(self.database) as db:
            db.execute("CREATE TABLE owner_evidence (id TEXT PRIMARY KEY, result TEXT)")
            db.execute(
                "INSERT INTO owner_evidence VALUES ('task-1', 'completed-before-upgrade')"
            )
            db.execute("PRAGMA user_version=3")
        self.database.chmod(0o600)
        self.original = {
            "hub_url": "http://127.0.0.1:17878",
            "worker_id": "work-pc",
            "database": str(self.database),
            "workspace_root": str(self.home / ".local/share/zaturobot/workspaces"),
            "artifact_root": str(self.home / ".local/share/zaturobot/artifacts"),
            "capabilities": ["diagnostic.v1", "agent.diagnostic.v1"],
            "max_concurrent_tasks": 1,
            "heartbeat_seconds": 5,
            "reconnect_max_seconds": 30,
            "models_config": str(self.models),
            "files": {
                "identity": str(self.identity),
                "inbox": str(self.home / ".local/share/zaturobot/inbox"),
                "cache": str(self.home / ".local/share/zaturobot/cache"),
            },
        }
        self.worker_config = write(
            self.config_dir / "worker.toml", updater.render_toml(self.original)
        )
        self.unit = write(
            self.home / ".config/systemd/user/zaturo-worker.service",
            (self.source / "deploy/notebook/zaturo-worker.user.service").read_bytes(),
        )
        self.old_worker = write(
            self.home / ".local/lib/zaturobot/zaturo-worker",
            "#!/bin/sh\n# old worker 0.3\n",
            0o700,
        )
        self.worker = write(
            self.base / "new-worker", "#!/bin/sh\n# verified new worker\n", 0o700
        )
        self.desktop = write(
            self.base / "new-desktop", "#!/bin/sh\n# verified new UI\n", 0o700
        )
        self.token = "synthetic-app-scoped-token-for-native-work-fixture"
        self.enrollment = write(
            self.base / "work.enrollment.json",
            json.dumps(
                {
                    "schema": 1,
                    "app_token": self.token,
                    "expected_zaturo_revision": source_revision,
                    "expected_youtube_revision": youtube_revision,
                }
            ),
        )
        self.arguments = [
            "--zaturo-root",
            str(self.source),
            "--youtube-root",
            str(self.youtube),
            "--home",
            str(self.home),
            "--enrollment",
            str(self.enrollment),
            "--no-start",
            "--skip-python-deps",
            "--skip-build",
            "--worker-binary",
            str(self.worker),
            "--desktop-binary",
            str(self.desktop),
            "--worker-sha256",
            updater.digest(self.worker.read_bytes()),
            "--desktop-sha256",
            updater.digest(self.desktop.read_bytes()),
        ]

    def tearDown(self):
        self.temporary.cleanup()

    def run_update(self, *arguments):
        output, errors = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            status = updater.main(self.arguments + list(arguments))
        for secret in (
            self.token,
            "synthetic-worker-secret-preserved",
            "synthetic-age-secret-preserved",
            "synthetic-ssh-secret-preserved",
        ):
            self.assertNotIn(secret, output.getvalue() + errors.getvalue())
        return status, json.loads(
            output.getvalue()
        ) if status == 0 else errors.getvalue()

    def test_dry_run_has_no_side_effects(self):
        before = {p: p.read_bytes() for p in self.home.rglob("*") if p.is_file()}
        status, report = self.run_update()
        self.assertEqual(status, 0, report)
        self.assertEqual(report["mode"], "dry_run")
        self.assertFalse(report["scheduling_granted"])
        self.assertEqual(
            before, {p: p.read_bytes() for p in self.home.rglob("*") if p.is_file()}
        )
        self.assertFalse((self.home / ".local/state").exists())

    def test_apply_preserves_identity_evidence_and_consistent_backup_and_is_idempotent(
        self,
    ):
        protected = {
            p: p.read_bytes()
            for p in (
                self.env,
                self.identity,
                self.ssh,
                self.models,
                self.database,
                self.unit,
            )
        }
        status, report = self.run_update("--apply", "--enable-scheduling")
        self.assertEqual(status, 0, report)
        self.assertEqual(report["mode"], "applied")
        self.assertFalse(report["started"])
        self.assertEqual(protected, {p: p.read_bytes() for p in protected})
        config = tomllib.loads(self.worker_config.read_text())
        for field in set(self.original) - {"capabilities"}:
            self.assertEqual(config[field], self.original[field])
        self.assertTrue(config["youtube"]["allow_schedule"])
        self.assertIn("youtube.submit.v1", config["capabilities"])
        self.assertEqual(config["files"]["identity"], str(self.identity))
        app_env = self.config_dir / "app.env"
        self.assertEqual(app_env.read_text(), "ZATURO_APP_TOKEN=" + self.token + "\n")
        self.assertEqual(stat.S_IMODE(app_env.stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(self.config_dir.stat().st_mode), 0o700)
        desktop = tomllib.loads((self.config_dir / "desktop.toml").read_text())
        self.assertEqual(desktop["target_worker"], "work-pc")
        self.assertEqual(desktop["transfer_identity"], str(self.identity))
        backup = Path(report["backup"])
        with sqlite3.connect(backup / "worker.db") as db:
            self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 3)
            self.assertEqual(
                db.execute("SELECT result FROM owner_evidence").fetchone()[0],
                "completed-before-upgrade",
            )
        manifest = json.loads((backup / "manifest.json").read_text())
        previous = {
            entry["path"]: (backup / entry["backup"]).read_bytes()
            for entry in manifest["files"]
        }
        self.assertEqual(
            previous[str(self.worker_config)], updater.render_toml(self.original)
        )
        self.assertEqual(
            previous[str(self.old_worker)], b"#!/bin/sh\n# old worker 0.3\n"
        )
        self.assertNotIn(str(self.identity), previous)
        for path in backup.rglob("*"):
            self.assertEqual(
                stat.S_IMODE(path.stat().st_mode), 0o700 if path.is_dir() else 0o600
            )
        status, repeated = self.run_update("--apply", "--enable-scheduling")
        self.assertEqual(status, 0, repeated)
        self.assertEqual(repeated["mode"], "already_current")
        self.assertEqual(len(list(backup.parent.iterdir())), 1)

    def test_explicit_grants_and_wrong_hash_or_dirty_pin_are_rejected(self):
        status, error = self.run_update("--apply", "--enable-private-view")
        self.assertNotEqual(status, 0)
        self.assertIn("explicitly supplied USB serial", error)
        self.arguments[self.arguments.index("--worker-sha256") + 1] = "0" * 64
        status, error = self.run_update("--apply")
        self.assertNotEqual(status, 0)
        self.assertIn("SHA-256", error)
        self.arguments[self.arguments.index("--worker-sha256") + 1] = updater.digest(
            self.worker.read_bytes()
        )
        write(self.source / "unexpected-source.py", "# unreviewed\n")
        status, error = self.run_update("--apply")
        self.assertNotEqual(status, 0)
        self.assertIn("source changes", error)
        self.assertFalse((self.home / ".local/state").exists())

    def test_permissions_and_symlink_destinations_are_rejected(self):
        self.enrollment.chmod(0o644)
        status, error = self.run_update()
        self.assertNotEqual(status, 0)
        self.assertIn("permissions", error)
        self.enrollment.chmod(0o600)
        victim = write(self.base / "outside-file", "preserved")
        (self.config_dir / "app.env").symlink_to(victim)
        status, error = self.run_update("--apply")
        self.assertNotEqual(status, 0)
        self.assertEqual(victim.read_text(), "preserved")
        self.assertEqual(
            self.worker_config.read_bytes(), updater.render_toml(self.original)
        )

    @unittest.skipUnless(Path("/usr/bin/adb").is_file(), "host has no adb binary")
    def test_explicit_usb_registration_is_local_grant_not_a_device_action(self):
        status, report = self.run_update(
            "--apply", "--android-serial", "SYNTHETIC_USB_123", "--enable-private-view"
        )
        self.assertEqual(status, 0, report)
        config = tomllib.loads(self.worker_config.read_text())
        self.assertTrue(config["android"]["allow_sensitive_views"])
        self.assertEqual(
            config["android"]["devices"],
            [
                {
                    "device_id": "owner-phone",
                    "adb_serial": "SYNTHETIC_USB_123",
                    "display_name": "Owner phone",
                    "capabilities": ["device_status", "sensitive_screen_view"],
                }
            ],
        )
        self.assertIn("android.private.v1", config["capabilities"])
        self.assertFalse(config["youtube"]["allow_schedule"])
        self.assertNotIn("youtube.submit.v1", config["capabilities"])

    def test_real_home_flow_stops_worker_before_backup_and_replacement(self):
        events = []
        active = True
        real_backup = updater.sqlite_backup
        real_write = updater.atomic_write

        def service(*arguments, allow_failure=False):
            nonlocal active
            events.append(arguments[0])
            if arguments[0] == "is-active":
                return active
            if arguments[0] == "stop":
                active = False
            if arguments[0] == "start":
                active = True
            return True

        def backup(source, target):
            self.assertFalse(active)
            events.append("sqlite-backup")
            return real_backup(source, target)

        def replace(path, content, mode=0o600):
            if path == self.old_worker:
                self.assertFalse(active)
                self.assertIn("sqlite-backup", events)
                events.append("worker-replacement")
            return real_write(path, content, mode)

        self.arguments.remove("--no-start")
        with (
            patch.object(updater.Path, "home", return_value=self.home),
            patch.object(updater, "systemctl", side_effect=service),
            patch.object(updater, "sqlite_backup", side_effect=backup),
            patch.object(updater, "atomic_write", side_effect=replace),
        ):
            status, report = self.run_update("--apply")
        self.assertEqual(status, 0, report)
        self.assertTrue(report["started"])
        self.assertTrue(report["previously_running"])
        self.assertLess(events.index("stop"), events.index("sqlite-backup"))
        self.assertLess(
            events.index("sqlite-backup"), events.index("worker-replacement")
        )
        self.assertLess(events.index("worker-replacement"), events.index("start"))

    def test_manual_process_is_not_bypassed_by_stopping_named_unit(self):
        original = self.old_worker.read_bytes()
        process = subprocess.Popen(
            [
                sys.executable,
                "-c",
                "import time; time.sleep(10)",
                "--config",
                str(self.worker_config),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            with (
                patch.object(updater.Path, "home", return_value=self.home),
                patch.object(
                    updater,
                    "systemctl",
                    side_effect=lambda *a, **k: a[0] != "is-active",
                ),
            ):
                status, error = self.run_update("--apply")
            self.assertNotEqual(status, 0)
            self.assertIn("live process", error)
            self.assertEqual(self.old_worker.read_bytes(), original)
            self.assertFalse(
                (self.home / ".local/state/zaturobot/work-services-backups").exists()
            )
        finally:
            process.terminate()
            process.wait(timeout=5)

    def test_already_current_no_start_still_quiesces_real_home(self):
        status, report = self.run_update("--apply")
        self.assertEqual(status, 0, report)
        calls = []
        active = True

        def service(*arguments, allow_failure=False):
            nonlocal active
            calls.append(arguments[0])
            if arguments[0] == "stop":
                active = False
            return active if arguments[0] == "is-active" else True

        with (
            patch.object(updater.Path, "home", return_value=self.home),
            patch.object(updater, "systemctl", side_effect=service),
        ):
            status, report = self.run_update("--apply")
        self.assertEqual(status, 0, report)
        self.assertEqual(report["mode"], "already_current")
        self.assertFalse(active)
        self.assertIn("stop", calls)
        self.assertNotIn("start", calls)

    def test_hub_or_unknown_journal_schemas_are_rejected_before_replacement(self):
        original = self.old_worker.read_bytes()
        for version in (1, 4, 5, 7):
            with sqlite3.connect(self.database) as database:
                database.execute(f"PRAGMA user_version={version}")
            status, error = self.run_update("--apply")
            self.assertNotEqual(status, 0)
            self.assertIn("journal schema", error)
            self.assertEqual(self.old_worker.read_bytes(), original)
            self.assertFalse((self.home / ".local/state").exists())

    def test_sqlite_backup_copies_committed_wal_records(self):
        with sqlite3.connect(self.database) as live:
            live.execute("PRAGMA journal_mode=WAL")
            live.execute(
                "INSERT INTO owner_evidence VALUES ('wal', 'committed-in-wal')"
            )
            live.commit()
            target = self.base / "consistent.db"
            updater.sqlite_backup(self.database, target)
            with sqlite3.connect(target) as backup:
                self.assertEqual(
                    backup.execute(
                        "SELECT result FROM owner_evidence WHERE id='wal'"
                    ).fetchone()[0],
                    "committed-in-wal",
                )

    def test_failed_write_after_backup_leaves_journal_and_recovery_evidence(self):
        original = self.database.read_bytes()
        real_write = updater.atomic_write

        def fail_config(path, content, mode=0o600):
            if path == self.worker_config:
                raise OSError("synthetic interrupted write")
            return real_write(path, content, mode)

        with patch.object(updater, "atomic_write", side_effect=fail_config):
            status, _ = self.run_update("--apply")
        self.assertNotEqual(status, 0)
        self.assertEqual(self.database.read_bytes(), original)
        recovery = list(
            (self.home / ".local/state/zaturobot/work-services-backups").glob(
                "*/UPDATE-INCOMPLETE.txt"
            )
        )
        self.assertEqual(len(recovery), 1)
        self.assertTrue((recovery[0].parent / "worker.db").is_file())


if __name__ == "__main__":
    unittest.main()
