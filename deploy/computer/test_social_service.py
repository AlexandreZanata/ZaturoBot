"""Synthetic Postiz Public API/approval/recovery contracts; no accounts or paid calls."""
from __future__ import annotations

import contextlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import stat
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch
import urllib.parse
import uuid

spec = importlib.util.spec_from_file_location("zaturo_social", Path(__file__).with_name("social_service.py"))
social = importlib.util.module_from_spec(spec)
spec.loader.exec_module(social)

NOW = 1_800_000_000


class FakeApi:
    def __init__(self, integration="account-1", platform="bluesky", disabled=False):
        self.integration = integration
        self.platform = platform
        self.disabled = disabled
        self.calls = []
        self.name = "Synthetic owner account"
        self.on_post = None
        self.on_integrations = None
        self.post_result = [{"postId": "post-123", "integration": integration}]
        self.posts = [{"id": "post-123", "state": "PUBLISHED", "integration": {"id": integration}}]

    def request(self, path, body=None):
        self.calls.append((path, body))
        if path == "/is-connected":
            return {"connected": True}
        if path == "/integrations":
            if self.on_integrations:
                self.on_integrations()
            return [{"id": self.integration, "identifier": self.platform,
                     "disabled": self.disabled, "name": self.name}]
        if path == "/posts":
            if self.on_post:
                self.on_post()
            return self.post_result
        if path.startswith("/posts?"):
            return {"posts": self.posts}
        raise AssertionError("unexpected fixed API route")


class Fixture(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="zaturo-social-fixture-")
        self.root = Path(self.temporary.name)
        self.key = "synthetic-postiz-application-key-for-tests-0001"
        self.key_file = self.root / "postiz.env"
        self.key_file.write_text("POSTIZ_API_KEY=" + self.key + "\n")
        self.key_file.chmod(0o600)
        self.profile = self.root / "social.toml"
        self.database = self.root / "social.json"
        self.api_base = "http://127.0.0.1:9/api/public/v1"
        self.allowed_origin = "http://127.0.0.1:9"
        self.allow_execute = True
        self.platform = "bluesky"
        self.media = {"media-1": self.allowed_origin + "/uploads/approved.png"}
        self.write_config()

    def tearDown(self):
        self.temporary.cleanup()

    def write_config(self):
        content = '\n'.join([
            'schema=1', 'enabled=true',
            'api_base=' + json.dumps(self.api_base),
            'allowed_origin=' + json.dumps(self.allowed_origin),
            'api_key_file=' + json.dumps(str(self.key_file)),
            'journal_file=' + json.dumps(str(self.database)),
            'allow_execute=' + str(self.allow_execute).lower(),
            '[allowed_integrations]', 'account-1=' + json.dumps(self.platform),
            '[allowed_media]',
            *[key + '=' + json.dumps(value) for key, value in self.media.items()],
        ]) + '\n'
        self.profile.write_text(content)
        self.profile.chmod(0o600)
        return social.Config(self.profile)

    def request(self, **updates):
        result = {"operation_id": str(uuid.uuid4()), "action": "publish", "integration_id": "account-1",
                  "content": "Publicação de teste sintético. Nenhuma conta real é usada."}
        result.update(updates)
        return result

    def approved(self, reply):
        return {"operation_id": reply["operation_id"], "approved_draft_hash": reply["draft_hash"], "owner_approved": True}

    def assert_code(self, code, function, *args):
        with self.assertRaises(social.SocialError) as error:
            function(*args)
        self.assertEqual(error.exception.code, code)


class SocialContracts(Fixture):
    def test_prepare_is_exact_inert_idempotent_and_cannot_overwrite_a_draft(self):
        config = self.write_config()
        request = self.request(media_ids=["media-1"])
        with patch.object(social.Api, "request", side_effect=AssertionError("prepare must not call HTTP")):
            reply = social.prepare(config, request, NOW)
            repeated = social.prepare(config, request, NOW + 2)
        self.assertEqual(reply, repeated)
        self.assertEqual(reply["status"], "prepared")
        self.assertEqual(reply["payload"], {"type": "now", "date": social.moment(NOW), "shortLink": False, "tags": [],
            "posts": [{"integration": {"id": "account-1"}, "value": [{"content": request["content"],
                "image": [{"id": "media-1", "path": self.media["media-1"]}]}], "settings": {"__type": "bluesky"}}]})
        self.assertIn(request["content"], reply["preview_pt"])
        self.assertNotIn(self.key, json.dumps(reply))
        self.assertLess(len(social.canonical(reply).encode()), social.MAX_OUTPUT)
        changed = {**request, "content": "Altered without another operation"}
        self.assert_code("draft_conflict", social.prepare, config, changed, NOW + 3)
        self.assertEqual(self.database.stat().st_mode & 0o777, 0o600)

    def test_exact_approval_local_grant_time_and_configuration_are_independent(self):
        config = self.write_config()
        reply = social.prepare(config, self.request(), NOW)
        api = FakeApi()
        wrong = {**self.approved(reply), "approved_draft_hash": "0" * 64}
        self.assert_code("owner_approval_required", social.execute, config, wrong, NOW + 1, api)
        self.assert_code("owner_approval_required", social.execute, config,
                         {**self.approved(reply), "owner_approved": False}, NOW + 1, api)
        self.assert_code("draft_expired", social.execute, config, self.approved(reply), NOW + social.MAX_DRAFT_AGE + 1, api)
        self.assert_code("clock_regressed", social.execute, config, self.approved(reply), NOW - 1, api)
        self.key_file.write_text("synthetic-rotated-key-owned-by-a-different-workspace\n")
        rotated = self.write_config()
        self.assert_code("config_changed", social.execute, rotated, self.approved(reply), NOW + 1, api)
        self.assertFalse(api.calls)

    def test_a_local_read_only_profile_does_not_grant_publication(self):
        self.allow_execute = False
        config = self.write_config()
        reply = social.prepare(config, self.request(), NOW)
        self.assert_code("local_write_grant_required", social.execute, config, self.approved(reply), NOW, FakeApi())

    def test_explicit_supported_schedule_and_draft_have_real_postiz_payload_types(self):
        config = self.write_config()
        request = self.request(action="schedule", scheduled_at=social.moment(NOW + 3600))
        reply = social.prepare(config, request, NOW)
        self.assertEqual(reply["payload"]["type"], "schedule")
        self.assertEqual(reply["payload"]["date"], request["scheduled_at"])
        draft = social.prepare(config, self.request(action="draft"), NOW)
        self.assertEqual(draft["payload"]["type"], "draft")
        self.assertNotEqual(draft["draft_hash"], reply["draft_hash"])
        self.assert_code("invalid_request", social.prepare, config,
                         self.request(action="schedule", scheduled_at="2026-11-14T12:00:00-04:00"), NOW)
        self.assert_code("invalid_request", social.prepare, config,
                         self.request(action="schedule", scheduled_at=social.moment(NOW + 10)), NOW)

    def test_api_acceptance_is_not_publication_and_cached_execute_never_posts_again(self):
        config = self.write_config()
        request = self.request()
        reply = social.prepare(config, request, NOW)
        api = FakeApi()
        accepted = social.execute(config, self.approved(reply), NOW + 1, api)
        self.assertEqual(accepted["status"], "accepted")
        self.assertNotIn("published", accepted)
        before = list(api.calls)
        self.assertEqual(social.prepare(config, request, NOW + 2), accepted,
            "An already submitted operation must not be described as an unsent draft")
        self.assertEqual(social.execute(config, self.approved(reply), NOW + 2, api), accepted)
        self.assertEqual(before, api.calls)
        observed = social.status(config, {"operation_id": request["operation_id"]}, api)
        self.assertEqual(observed["status"], "published")
        self.assertFalse(observed["screenshot_created"])
        self.assertEqual(sum(path == "/posts" for path, _ in api.calls), 1)
        query = urllib.parse.parse_qs(urllib.parse.urlsplit(api.calls[-1][0]).query)
        self.assertIn("startDate", query)
        self.assertIn("endDate", query)
        api.posts = [{"id": "post-123", "state": "PUBLISHED", "integration": {"id": "another-account"}}]
        self.assertEqual(social.status(config, {"operation_id": request["operation_id"]}, api)["code"], "post_not_observed")

    def test_unknown_write_response_stays_fenced_across_reopen_and_new_operation(self):
        config = self.write_config()
        request = self.request()
        reply = social.prepare(config, request, NOW)
        api = FakeApi()
        api.on_post = lambda: (_ for _ in ()).throw(social.ApiError("api_unavailable"))
        outcome = social.execute(config, self.approved(reply), NOW + 1, api)
        self.assertEqual(outcome["code"], "reconciliation_required")
        self.assertEqual(outcome["provider_code"], "api_unavailable")
        before = list(api.calls)
        reopened = social.Config(self.profile)
        self.assert_code("reconciliation_required", social.execute, reopened, self.approved(reply), NOW + 2, api)
        self.assertEqual(social.status(reopened, {"operation_id": request["operation_id"]}, api)["code"], "reconciliation_required")
        second = social.prepare(reopened, self.request(), NOW + 3)
        self.assert_code("integration_reconciliation_required", social.execute, reopened, self.approved(second), NOW + 4, api)
        self.assertEqual(before, api.calls)

    def test_interrupted_process_keeps_durable_intent_without_automatic_retry(self):
        config = self.write_config()
        request = self.request()
        reply = social.prepare(config, request, NOW)
        api = FakeApi()
        api.on_post = lambda: (_ for _ in ()).throw(KeyboardInterrupt())
        with self.assertRaises(KeyboardInterrupt):
            social.execute(config, self.approved(reply), NOW + 1, api)
        before = list(api.calls)
        self.assert_code("reconciliation_required", social.execute, social.Config(self.profile), self.approved(reply), NOW + 2, api)
        self.assertEqual(before, api.calls)

    def test_a_response_journal_failure_keeps_the_committed_intent_as_a_fence(self):
        config = self.write_config()
        request = self.request()
        reply = social.prepare(config, request, NOW)
        api = FakeApi()
        commit = social.Journal.commit
        def failing_response(journal):
            if any(row["state"] in {"accepted", "uncertain"} for row in journal.state["operations"].values()):
                raise OSError("synthetic response persistence failure")
            return commit(journal)
        with patch.object(social.Journal, "commit", failing_response):
            outcome = social.execute(config, self.approved(reply), NOW + 1, api)
        self.assertEqual(outcome["code"], "reconciliation_required")
        self.assertEqual(sum(path == "/posts" for path, _ in api.calls), 1)
        self.assert_code("reconciliation_required", social.execute, config, self.approved(reply), NOW + 2, api)
        self.assertEqual(social.prepare(config, request, NOW + 2)["code"], "reconciliation_required")
        self.assertEqual(sum(path == "/posts" for path, _ in api.calls), 1)

    def test_another_operation_cannot_cross_an_inflight_account_intent(self):
        config = self.write_config()
        first = social.prepare(config, self.request(), NOW)
        second = social.prepare(config, self.request(content="Another explicitly reviewed test post"), NOW)
        first_api, second_api = FakeApi(), FakeApi()
        def concurrent_attempt():
            self.assert_code("integration_reconciliation_required", social.execute,
                config, self.approved(second), NOW + 2, second_api)
        first_api.on_post = concurrent_attempt
        self.assertEqual(social.execute(config, self.approved(first), NOW + 1, first_api)["status"], "accepted")
        self.assertFalse(second_api.calls)
        self.assertEqual(sum(path == "/posts" for path, _ in first_api.calls), 1)

    def test_revoked_local_grant_and_elapsed_expiry_are_rechecked_after_preflight(self):
        config = self.write_config()
        reply = social.prepare(config, self.request(), NOW)
        api = FakeApi()
        def revoke():
            self.allow_execute = False
            self.write_config()
        api.on_integrations = revoke
        self.assert_code("config_changed", social.execute, config, self.approved(reply), NOW + 1, api)
        self.assertFalse(any(path == "/posts" for path, _ in api.calls))
        self.allow_execute = True
        config = self.write_config()
        api = FakeApi()
        with patch.object(social.time, "monotonic", side_effect=[0, social.MAX_DRAFT_AGE + 5]):
            self.assert_code("draft_expired", social.execute, config, self.approved(reply), NOW + 1, api)
        self.assertFalse(any(path == "/posts" for path, _ in api.calls))

    def test_readiness_does_not_invent_a_network_login_and_refuses_missing_or_disabled_accounts(self):
        config = self.write_config()
        api = FakeApi()
        observed = social.readiness(config, {}, api)
        self.assertEqual(observed["accounts"][0]["state"], "registered")
        self.assertIn("não comprova login", observed["summary_pt"])
        api.name = self.key
        self.assertNotIn(self.key, json.dumps(social.readiness(config, {}, api)))
        class MalformedConnection(FakeApi):
            def request(inner, path, body=None):
                return inner.connection if path == "/is-connected" else super().request(path, body)
        malformed = MalformedConnection()
        for value in [{}, {"connected": "true"}, {"connected": 1}, []]:
            malformed.connection = value
            self.assert_code("api_unavailable", social.readiness, config, {}, malformed)
        malformed.connection = {"connected": False}
        self.assert_code("api_authentication", social.readiness, config, {}, malformed)
        reply = social.prepare(config, self.request(), NOW)
        self.assert_code("integration_disabled", social.execute, config, self.approved(reply), NOW, FakeApi(disabled=True))
        self.assert_code("integration_missing", social.execute, config, self.approved(reply), NOW, FakeApi(integration="another-account"))
        self.assert_code("integration_unknown", social.execute, config, self.approved(reply), NOW, FakeApi(platform="mastodon"))

    def test_unimplemented_actions_platforms_and_unregistered_media_fail_without_http(self):
        config = self.write_config()
        self.assert_code("unsupported_action", social.prepare, config, self.request(action="comment_reply"), NOW)
        self.assert_code("integration_not_allowed", social.prepare, config, self.request(integration_id="another-account"), NOW)
        self.assert_code("media_not_allowed", social.prepare, config, self.request(media_ids=["https://127.0.0.1/private"]), NOW)
        self.assert_code("invalid_request", social.prepare, config, {**self.request(), "shell": "unrestricted"}, NOW)
        self.platform = "youtube"
        self.assert_code("unsupported_platform", social.prepare, self.write_config(), self.request(), NOW)
        self.assertFalse(self.database.exists())
        self.assertIn("comment_reply", social.capabilities()["unsupported"])
        self.platform = "bluesky"
        config = self.write_config()
        self.assert_code("invalid_request", social.prepare, config, self.request(content=self.key), NOW)
        api = FakeApi()
        api.post_result = [{"postId": self.key, "integration": "account-1"}]
        draft = social.prepare(config, self.request(), NOW)
        reply = social.execute(config, self.approved(draft), NOW + 1, api)
        self.assertEqual(reply["code"], "reconciliation_required")
        self.assertNotIn(self.key, json.dumps(reply))

    def test_private_profile_key_endpoint_and_media_origins_fail_closed(self):
        self.key_file.chmod(0o644)
        self.assert_code("invalid_config", social.Config, self.profile)
        self.key_file.chmod(0o600)
        original = self.key_file.read_bytes()
        self.key_file.unlink()
        other = self.root / "other-key"
        other.write_bytes(original)
        other.chmod(0o600)
        self.key_file.symlink_to(other)
        self.assert_code("invalid_config", social.Config, self.profile)
        self.key_file.unlink()
        self.key_file.write_bytes(original)
        self.key_file.chmod(0o600)
        self.allowed_origin = "http://postiz.example.com"
        self.assert_code("invalid_config", self.write_config)
        self.allowed_origin = "http://127.0.0.1:9"
        self.media = {"arbitrary": "http://169.254.169.254/latest/meta-data/"}
        self.assert_code("invalid_config", self.write_config)

    def test_atomic_receipts_sync_file_before_replace_and_directory_after(self):
        config = self.write_config()
        events = []
        sync, replace = os.fsync, os.replace
        def observed_sync(descriptor):
            events.append("directory_fsync" if stat.S_ISDIR(os.fstat(descriptor).st_mode) else "file_fsync")
            return sync(descriptor)
        def observed_replace(source, destination):
            self.assertEqual(Path(destination), self.database)
            events.append("replace")
            return replace(source, destination)
        with patch.object(social.os, "fsync", observed_sync), patch.object(social.os, "replace", observed_replace):
            social.prepare(config, self.request(), NOW)
        self.assertEqual(events, ["file_fsync", "replace", "directory_fsync"])
        self.assertFalse(list(self.root.glob(".social-receipt-*")))
        self.assertEqual(self.database.with_name(self.database.name + ".lock").stat().st_mode & 0o777, 0o600)

    def test_corrupted_owned_receipts_are_preserved_and_never_used_as_permission(self):
        config = self.write_config()
        reply = social.prepare(config, self.request(), NOW)
        receipt = json.loads(self.database.read_text())
        row = receipt["operations"][reply["operation_id"]]
        body = json.loads(row["body_json"])
        body["posts"][0]["value"][0]["content"] = "Changed without the reviewed hash"
        row["body_json"] = social.canonical(body)
        self.database.write_text(json.dumps(receipt))
        before = self.database.read_bytes()
        api = FakeApi()
        self.assert_code("local_state_unavailable", social.execute, config, self.approved(reply), NOW + 1, api)
        self.assertFalse(api.calls)
        self.assertEqual(self.database.read_bytes(), before)

    def test_bounded_record_capacity_does_not_delete_an_existing_operation(self):
        config = self.write_config()
        social.prepare(config, self.request(), NOW)
        before = self.database.read_bytes()
        with patch.object(social, "MAX_OPERATIONS", 1):
            self.assert_code("local_capacity", social.prepare, config, self.request(), NOW + 1)
        self.assertEqual(self.database.read_bytes(), before)

    def test_foreign_local_files_are_preserved_and_json_is_unambiguous(self):
        self.database.write_text('{"owner_evidence":["preserve"]}\n')
        self.database.chmod(0o600)
        before = self.database.read_bytes()
        config = self.write_config()
        self.assert_code("local_state_unavailable", social.prepare, config, self.request(), NOW)
        self.assertEqual(self.database.read_bytes(), before)
        self.assert_code("invalid_request", social.json_object, '{"operation_id":"one","operation_id":"two"}')
        self.assert_code("invalid_request", social.json_object, '{"value":NaN}')


@contextlib.contextmanager
def server_fixture(key):
    state = {"calls": [], "mode": "normal"}
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass
        def do_GET(self):
            state["calls"].append((self.command, self.path, self.headers.get("Authorization")))
            if state["mode"] == "redirect":
                self.send_response(302)
                self.send_header("Location", "/credential-sink")
                self.end_headers()
                return
            if state["mode"] == "error":
                self.send_response(401)
                self.end_headers()
                self.wfile.write(("private provider error " + key).encode())
                return
            if self.path == "/api/public/v1/is-connected":
                body = {"connected": True}
            elif self.path == "/api/public/v1/integrations":
                body = [{"id": "account-1", "name": "Synthetic HTTP account", "identifier": "bluesky", "disabled": False}]
            elif self.path.startswith("/api/public/v1/posts?"):
                body = {"posts": [{"id": "http-post-123", "state": "PUBLISHED", "integration": {"id": "account-1"}}]}
            else:
                raise AssertionError("unexpected API route")
            self.send_json(body)
        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            state["calls"].append((self.command, self.path, self.headers.get("Authorization"), body))
            assert self.path == "/api/public/v1/posts"
            assert body["posts"][0]["integration"]["id"] == "account-1"
            assert body["type"] == "now"
            self.send_json([{"postId": "http-post-123", "integration": "account-1"}])
        def send_json(self, body):
            data = json.dumps(body).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    state["origin"] = "http://127.0.0.1:" + str(server.server_port)
    try:
        yield state
    finally:
        server.shutdown()
        server.server_close()
        thread.join(3)


class HttpContracts(Fixture):
    def configure_server(self, state):
        self.allowed_origin = state["origin"]
        self.api_base = self.allowed_origin + "/api/public/v1"
        self.media = {}
        return self.write_config()

    def test_real_supported_routes_and_raw_authorization_with_proxy_disabled(self):
        with server_fixture(self.key) as state:
            config = self.configure_server(state)
            with patch.dict(os.environ, {"HTTP_PROXY": "http://127.0.0.1:1", "HTTPS_PROXY": "http://127.0.0.1:1", "NO_PROXY": ""}):
                reply = social.prepare(config, self.request(), NOW)
                accepted = social.execute(config, self.approved(reply), NOW)
                self.assertEqual(accepted["status"], "accepted")
                observed = social.status(config, {"operation_id": reply["operation_id"]})
            self.assertEqual(observed["status"], "published")
            self.assertTrue(all(call[2] == self.key for call in state["calls"]))
            self.assertEqual(sum(call[0] == "POST" for call in state["calls"]), 1)
            self.assertTrue(any(call[1].startswith("/api/public/v1/posts?startDate=") for call in state["calls"]))

    def test_redirects_never_forward_credentials_and_raw_provider_errors_stay_private(self):
        with server_fixture(self.key) as state:
            config = self.configure_server(state)
            state["mode"] = "redirect"
            self.assert_code("api_rejected", social.readiness, config, {})
            self.assertEqual(len(state["calls"]), 1)
            self.assertNotEqual(state["calls"][0][1], "/credential-sink")
            state["mode"] = "error"
            try:
                social.readiness(config, {})
            except social.ApiError as error:
                result = social.blocked(error.code)
            else:
                self.fail("Expected API authentication error")
            self.assertEqual(result["code"], "api_authentication")
            self.assertNotIn(self.key, json.dumps(result))
            self.assertNotIn("private provider error", json.dumps(result))

    def test_stdio_protocol_returns_only_bounded_typed_json(self):
        helper = Path(__file__).with_name("social_service.py")
        result = subprocess.run([sys.executable, str(helper), "capabilities"], input=b"{}", capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0)
        value = json.loads(result.stdout)
        self.assertEqual(value["schema"], 1)
        self.assertFalse(result.stderr)
        request = self.request(content="POSTIZ_API_KEY=synthetic-do-not-echo")
        result = subprocess.run([sys.executable, str(helper), "prepare", "--config", str(self.profile)],
            input=json.dumps(request).encode(), capture_output=True, timeout=5)
        value = json.loads(result.stdout)
        self.assertEqual(value["code"], "invalid_request")
        self.assertNotIn("synthetic-do-not-echo", result.stdout.decode())
        self.assertFalse(result.stderr)
        self.assertFalse(self.database.exists())


if __name__ == "__main__":
    unittest.main()
