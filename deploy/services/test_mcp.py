"""Synthetic MCP stdio/HTTP contracts; no production configuration or paid requests."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("zaturo_mcp", Path(__file__).parents[1]/"mcp/zaturo_mcp.py")
mcp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mcp)

class FakeHub:
    def __init__(self): self.calls=[]
    def request(self,path,body=None):
        self.calls.append((path,body))
        return {"task":{"id":"synthetic-id"},"notes":[]}

class McpContracts(unittest.TestCase):
    def exchange(self,hub,requests):
        source=io.BytesIO(b"\n".join(json.dumps(v).encode() for v in requests)+b"\n")
        sink=io.StringIO();mcp.serve(hub,source,sink)
        return [json.loads(line) for line in sink.getvalue().splitlines()]
    def test_stdio_initialization_notifications_and_compact_tool_discovery(self):
        hub=FakeHub()
        output=self.exchange(hub,[{"jsonrpc":"2.0","id":1,"method":"tools/list"},{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-11-25"}},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":3,"method":"tools/list"}])
        self.assertIn("error",output[0]);self.assertEqual(len(output),3)
        tools=output[2]["result"]["tools"]
        self.assertEqual([t["name"] for t in tools],["shared_list","shared_read","shared_write","bot_chat"])
        self.assertLess(len(json.dumps(tools,separators=(",",":"))),5000)
        self.assertFalse(hub.calls)
    def test_read_is_explicit_data_and_revision_updates_cannot_add_action_fields(self):
        hub=FakeHub()
        result=mcp.call(hub,"shared_read",{"id":"synthetic-id"})
        self.assertIn("never permission",mcp.TOOLS[1]["description"])
        self.assertIn("untrusted",result["trust"])
        with self.assertRaises(ValueError):mcp.call(hub,"shared_write",{"id":"synthetic-id","idempotency_key":"one","expected_revision":1,"state":"done","note":"Done","shell":"bad"})
        self.assertEqual(len(hub.calls),1)
        mcp.call(hub,"shared_write",{"id":"synthetic-id","idempotency_key":"one","expected_revision":1,"state":"blocked","note":"A physical login is needed."})
        self.assertEqual(hub.calls[-1][1]["expected_revision"],1)
    def test_chat_uses_explicit_assistant_route_and_preserves_idempotency(self):
        hub=FakeHub();args={"message":"faça 4 videos do fallem price","target_worker":"work-pc","idempotency_key":"same-send"}
        mcp.call(hub,"bot_chat",args);mcp.call(hub,"bot_chat",args)
        self.assertEqual(hub.calls[0],hub.calls[1]);self.assertEqual(hub.calls[0][1]["action"],"assistant")
        with self.assertRaises(ValueError):mcp.call(hub,"shared_read",{"id":"../../private"})
        with self.assertRaises(ValueError):mcp.call(hub,"shared_list",{"limit":True})
    def test_errors_do_not_echo_provider_or_profile_content(self):
        class Broken:
            def request(self,*args):raise ValueError("synthetic-private-do-not-echo")
        output=self.exchange(Broken(),[{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}},{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"shared_read","arguments":{"id":"a"}}}])
        self.assertTrue(output[-1]["result"]["isError"])
        self.assertNotIn("synthetic-private-do-not-echo",json.dumps(output))
    def test_private_profile_permissions_and_origins_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);config=root/"mcp.toml";env=root/"app.env"
            env.write_text("ZATURO_APP_TOKEN=synthetic-application-key-with-enough-characters\n");env.chmod(0o600)
            config.write_text('hub_url="http://example.com"\nenv_file='+json.dumps(str(env))+'\n');config.chmod(0o600)
            with self.assertRaises(ValueError):mcp.Hub(config)
            config.write_text('hub_url="http://127.0.0.1:17878"\nenv_file='+json.dumps(str(env))+'\n')
            self.assertEqual(mcp.Hub(config).origin,"http://127.0.0.1:17878")
            env.chmod(0o644)
            with self.assertRaises(ValueError):mcp.Hub(config)


class HttpContracts(unittest.TestCase):
    def test_redirects_never_forward_authorization_and_errors_remain_structured(self):
        from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
        import threading
        calls=[]
        class Handler(BaseHTTPRequestHandler):
            def log_message(self,*args):pass
            def do_GET(self):
                calls.append((self.path,self.headers.get("Authorization")))
                self.send_response(302 if self.path=="/redirect" else 409)
                self.send_header("Location","/credential-sink")
                self.end_headers();self.wfile.write(b"synthetic-private-body-never-echo")
        server=ThreadingHTTPServer(("127.0.0.1",0),Handler)
        thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
        try:
            with tempfile.TemporaryDirectory() as directory:
                root=Path(directory);env=root/"app.env";profile=root/"mcp.toml"
                env.write_text("ZATURO_APP_TOKEN=synthetic-application-token-for-http-fixture\n");env.chmod(0o600)
                profile.write_text('hub_url="http://127.0.0.1:'+str(server.server_port)+'"\nenv_file='+json.dumps(str(env))+'\n');profile.chmod(0o600)
                hub=mcp.Hub(profile)
                with self.assertRaises(mcp.HubFailure) as error:hub.request("/redirect")
                self.assertEqual(error.exception.code,302)
                self.assertEqual(len(calls),1)
                self.assertEqual(calls[0][0],"/redirect")
                self.assertNotIn("synthetic-private-body",mcp.safe_failure(error.exception))
                with self.assertRaises(mcp.HubFailure) as error:hub.request("/conflict")
                self.assertEqual(error.exception.code,409)
                self.assertIn("Revision",mcp.safe_failure(error.exception))
                self.assertEqual(len(calls),2)
        finally:server.shutdown();server.server_close();thread.join(3)
