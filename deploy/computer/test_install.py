"""Offline installer acceptance: preservation, explicit local stop and verified artifacts."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import tomllib
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
spec = importlib.util.spec_from_file_location("zaturo_computer_install", Path(__file__).with_name("install.py"))
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)


class InstallContracts(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="zaturo-computer-install-fixture-")
        self.root = Path(self.temp.name)
        self.home = self.root / "home"; self.home.mkdir(mode=0o700)
        self.source = self.root / "source"
        self.helpers = self.source / "deploy/computer"; self.helpers.mkdir(parents=True)
        for name in installer.HELPERS:
            path = self.helpers / name
            path.write_text("# Synthetic reviewed helper: " + name + "\n")
            path.chmod(0o664)
        self.config = self.home / ".config/zaturobot"
        self.lib = self.home / ".local/lib/zaturobot/computer"
        self.state = self.home / ".local/state/zaturobot/computer"
        self.artifact = self.root / "computer-use-linux"
        self.artifact.write_bytes(b"Synthetic verified backend; never executed")
        self.artifact.chmod(0o700)
        self.license = self.root / "LICENSE"
        self.license.write_text("Synthetic pinned MIT license for fixture\n")
        self.license.chmod(0o600)
        self.artifact_hash = hashlib.sha256(self.artifact.read_bytes()).hexdigest()
        self.license_hash = hashlib.sha256(self.license.read_bytes()).hexdigest()

    def tearDown(self): self.temp.cleanup()
    def install(self, **options):
        return installer.install(self.home, source_root=self.source, **options)
    def artifact_options(self):
        return {"backend_binary": self.artifact, "backend_sha256": self.artifact_hash, "backend_license": self.license}
    def artifact_patches(self):
        return patch.multiple(installer, RELEASE_HASHES={installer.platform.machine(): {"gnu": self.artifact_hash, "musl": "0" * 64}}, LICENSE_SHA256=self.license_hash)
    def assert_private(self, path, mode):
        self.assertEqual(stat.S_IMODE(path.stat().st_mode), mode)
    def assert_rejected(self, code, **options):
        with self.assertRaises(installer.bridge.Rejected) as error: self.install(**options)
        self.assertEqual(error.exception.code, code)

    def test_plan_is_read_only_and_apply_creates_disabled_private_profiles(self):
        credentials = self.home / "owner-credentials.env"
        credentials.write_bytes(b"fixture-private-preserve-exactly\n"); credentials.chmod(0o600)
        before = list(self.home.rglob("*"))
        plan = self.install()
        self.assertEqual(plan["mode"], "plan"); self.assertEqual(list(self.home.rglob("*")), before)
        self.assertFalse(plan["grants_enabled_by_installer"])
        self.assertFalse(plan["desktop_or_accounts_contacted"])
        self.assertNotIn("fixture-private", json.dumps(plan))
        report = self.install(apply=True)
        self.assertEqual(report["mode"], "applied")
        for name in installer.HELPERS:
            self.assertEqual((self.lib / name).read_bytes(), (self.helpers / name).read_bytes())
            self.assert_private(self.lib / name, 0o700)
        for name in ("computer-linux.toml", "computer-browser.toml"):
            profile = tomllib.loads((self.config / name).read_text())
            self.assertFalse(profile["enabled"]); self.assertFalse(profile["allow_input"]); self.assertFalse(profile["allow_capture"])
            self.assert_private(self.config / name, 0o600)
        self.assertEqual(tomllib.loads((self.config / "computer-linux.toml").read_text())["allowed_apps"], [])
        self.assertEqual(tomllib.loads((self.config / "computer-browser.toml").read_text())["playwright_tree_sha256"], "")
        self.assertEqual(credentials.read_bytes(), b"fixture-private-preserve-exactly\n")
        self.assert_private(self.lib, 0o700); self.assert_private(self.config, 0o700)
        self.assert_private(self.lib.parent, 0o700); self.assert_private(self.home / ".local", 0o700)

    def test_existing_profile_and_credentials_preserved_and_helper_backup_is_recoverable(self):
        self.install(apply=True)
        profile = self.config / "computer-linux.toml"
        original = b"# Owner configuration retained without parsing\nenabled=true\nprivate_note='fixture-owner-data'\n"
        profile.write_bytes(original); profile.chmod(0o600)
        credentials = self.config / "private.env"
        credentials.write_bytes(b"fixture-private-credentials\n"); credentials.chmod(0o600)
        old_helper = (self.lib / installer.HELPERS[0]).read_bytes()
        (self.helpers / installer.HELPERS[0]).write_text("# New reviewed fixture version\n")
        plan = self.install()
        self.assertEqual(plan["existing_profiles_preserved"], [str(self.config / name) for name in ("computer-linux.toml", "computer-browser.toml")])
        self.assertNotIn("fixture-owner-data", json.dumps(plan))
        report = self.install(apply=True)
        self.assertEqual(profile.read_bytes(), original)
        self.assertEqual(credentials.read_bytes(), b"fixture-private-credentials\n")
        backup = Path(report["backup"]); self.assert_private(backup, 0o700)
        manifest = json.loads((backup / "manifest.json").read_text())
        self.assertEqual(len(manifest), 1)
        self.assertEqual(manifest[0]["path"], str(self.lib / installer.HELPERS[0]))
        saved = backup / manifest[0]["backup"]
        self.assertEqual(saved.read_bytes(), old_helper); self.assert_private(saved, 0o600)
        self.assertEqual(manifest[0]["sha256"], hashlib.sha256(old_helper).hexdigest())
        repeated = self.install(apply=True)
        self.assertIsNone(repeated["backup"])
        self.assertEqual(len(list(backup.parent.iterdir())), 1)

    def test_stop_is_preserved_by_install_and_only_explicit_local_resume_clears_it(self):
        self.install(apply=True, pause="desktop")
        marker = self.state / "desktop/STOP"
        self.assertEqual(marker.read_bytes(), installer.STOP_CONTENT); self.assert_private(marker, 0o600)
        owner_note = b"Owner local pause; preserve this exact text\n"
        marker.write_bytes(owner_note); marker.chmod(0o600)
        self.install(apply=True); self.install(apply=True, pause="desktop")
        self.assertEqual(marker.read_bytes(), owner_note)
        self.install(resume="desktop")
        self.assertEqual(marker.read_bytes(), owner_note)
        self.install(apply=True, resume="desktop")
        self.assertFalse(marker.exists())
        profile = tomllib.loads((self.config / "computer-linux.toml").read_text())
        self.assertFalse(profile["enabled"]); self.assertFalse(profile["allow_input"])
        self.install(apply=True, pause="browser")
        self.assertTrue((self.state / "browser/STOP").exists())
        self.assertFalse((self.state / "desktop/STOP").exists())
        self.assert_rejected("invalid_stop_request", apply=True, pause="browser", resume="browser")

    def test_official_pinned_artifact_and_license_are_copied_without_execution(self):
        with self.artifact_patches(), patch.object(installer, "atomic", wraps=installer.atomic) as writes:
            report = self.install(apply=True, **self.artifact_options())
            self.assertTrue(writes.called)
        binary = Path(report["backend"]["destination"])
        self.assertEqual(binary.read_bytes(), self.artifact.read_bytes()); self.assert_private(binary, 0o700)
        license_path = Path(report["backend"]["license"])
        self.assertEqual(license_path.read_bytes(), self.license.read_bytes()); self.assert_private(license_path, 0o600)
        notice = json.loads((license_path.parent / "NOTICE.json").read_text())
        self.assertEqual(notice["artifact_sha256"], self.artifact_hash)
        self.assertEqual(notice["source_commit"], installer.bridge.UPSTREAM_COMMIT)
        profile = tomllib.loads((self.config / "computer-linux.toml").read_text())
        self.assertEqual(profile["binary_sha256"], self.artifact_hash); self.assertFalse(profile["enabled"])
        with self.artifact_patches(): self.assertTrue(self.install()["backend"]["installed"])

    def test_bad_artifacts_and_source_symlinks_fail_before_any_mutation(self):
        self.assert_rejected("complete_verified_artifact_required", apply=True, backend_binary=self.artifact)
        self.assertEqual(list(self.home.iterdir()), [])
        with self.artifact_patches():
            self.artifact.write_bytes(b"Tampered backend")
            self.assert_rejected("artifact_hash_mismatch", apply=True, **self.artifact_options())
            self.assertEqual(list(self.home.iterdir()), [])
        helper = self.helpers / installer.HELPERS[-1]
        helper.unlink(); helper.symlink_to(self.helpers / installer.HELPERS[0])
        self.assert_rejected("unsafe_install_path", apply=True)
        self.assertEqual(list(self.home.iterdir()), [])

    def test_destination_and_stop_symlinks_cannot_replace_owner_files(self):
        self.install(apply=True)
        victim = self.root / "owner-file"; victim.write_bytes(b"preserve"); victim.chmod(0o600)
        profile = self.config / "computer-linux.toml"
        profile.unlink(); profile.symlink_to(victim)
        self.assert_rejected("unsafe_install_path", apply=True)
        self.assertEqual(victim.read_bytes(), b"preserve")
        profile.unlink(); self.install(apply=True)
        state = self.state / "desktop"; state.mkdir(mode=0o700)
        (state / "STOP").symlink_to(victim)
        self.assert_rejected("unsafe_install_path", apply=True, resume="desktop")
        self.assertEqual(victim.read_bytes(), b"preserve")

    def test_world_writable_source_and_group_readable_private_profile_are_refused(self):
        helper = self.helpers / installer.HELPERS[0]; helper.chmod(0o666)
        self.assert_rejected("unsafe_install_file", apply=True)
        self.assertEqual(list(self.home.iterdir()), [])
        helper.chmod(0o664); self.install(apply=True)
        (self.config / "computer-linux.toml").chmod(0o640)
        self.assert_rejected("unsafe_install_file", apply=True)

    def test_same_bytes_helper_mode_is_normalized_with_recoverable_backup(self):
        self.install(apply=True)
        helper = self.lib / installer.HELPERS[0]
        original = helper.read_bytes(); helper.chmod(0o644)
        report = self.install(apply=True)
        self.assertEqual(helper.read_bytes(), original); self.assert_private(helper, 0o700)
        backup = Path(report["backup"])
        manifest = json.loads((backup / "manifest.json").read_text())
        self.assertEqual(manifest[0]["mode"], 0o644)
        self.assertEqual((backup / manifest[0]["backup"]).read_bytes(), original)
        self.assertIsNone(self.install(apply=True)["backup"])

    def test_explicit_stop_uses_preserved_custom_profile_state_not_default(self):
        self.install(apply=True)
        custom=self.home/'custom-private-state';custom.mkdir(mode=0o700)
        profile=self.config/'computer-linux.toml'
        content='enabled=false\nstate_dir='+json.dumps(str(custom))+'\n'
        profile.write_text(content);profile.chmod(0o600)
        report=self.install(apply=True,pause='desktop')
        self.assertEqual(report['stop_action']['marker'],str(custom/'STOP'))
        self.assertTrue((custom/'STOP').is_file())
        self.assertFalse((self.state/'desktop/STOP').exists())
        self.assertEqual(profile.read_text(),content)
        self.install(apply=True,resume='desktop')
        self.assertFalse((custom/'STOP').exists())
        profile.write_text('enabled=false\n')
        self.assert_rejected('invalid_stop_profile',apply=True,pause='desktop')
        self.assertFalse((self.state/'desktop/STOP').exists())


if __name__ == "__main__": unittest.main()
