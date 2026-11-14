# Compact local ZaturoBot MCP

`zaturo_mcp.py` is a Python 3.11+ standard-library stdio server. It uses newline-delimited
JSON-RPC and negotiates the legacy MCP 2025-11-25 shape. There is no new network listener
or public port. The existing pinned SSH tunnel/private HTTPS hub remains the transport.
[Official MCP stdio](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
and [tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) are the
wire references. It intentionally exposes only four stable tools:

- `shared_list`: up to 20 summaries, default 10, with target/state/page filters.
- `shared_read`: one body and recent revision notes; content is explicitly untrusted data.
- `shared_write`: create an inert handoff/transfer references or append a revisioned update.
- `bot_chat`: submit the owner's explicit bounded assistant request with a stable key.

List summaries before reading a body. There are no full-history dumps, image/base64 tool
results, provider prompts on each tool call, generic shell, automatic file execution or
model key. The compact tool catalog is tested below 5000 JSON bytes; byte size is not a
claim about a provider tokenizer. Known chat commands need zero inference calls. Other chat uses the home worker's explicit
Personal/Go opt-in and existing budgets; the MCP adapter receives no provider key, cannot
select a model/route or turn on credit fallback. Go live billing remains separate evidence. See
[assistant contracts](../../docs/ASSISTANT_AND_SHARED_TASKS.md).

Computer confirmations require the native owner's separate `computer_confirm` action.
`bot_chat` always uses `assistant`, rejects approval commands and cannot approve its own
computer inputs or social publications, including through an older MCP adapter.

## Install on an already configured Linux PC

Use the clean reviewed source revision. The installer reads the existing private
`~/.config/zaturobot/desktop.toml`, validates an application-only credential profile and
prints a plan. It never imports an admin/model key or changes the worker/tunnel:

```bash
python3 deploy/mcp/install.py
python3 deploy/mcp/install.py --apply
```

It installs the adapter under `~/.local/lib/zaturobot/` and mode0600 `mcp.toml` with only
`hub_url` and `env_file`. The existing `app.env` stays separate/private and must contain
`ZATURO_APP_TOKEN`. Symlinks, broad permissions, non-loopback cleartext origins and
redirects fail closed. Conflicting adapter/profile versions are privately backed up.
The work-services updater also installs this adapter/profile when applying a service update.

The installer prints the exact credential-free registration command for that PC:

```bash
codex mcp add zaturobot -- /usr/bin/python3 /home/owner/.local/lib/zaturobot/zaturo_mcp.py --config /home/owner/.config/zaturobot/mcp.toml
```

Use the actual paths from the plan. Restart/reload the MCP connection in the receiving
Codex and test `shared_list` against the existing private tunnel. The official
[Codex MCP documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli) describes
stdio server registration and configuration. This setup does not run Codex automatically,
create a new Codex chat, or grant permission to execute shared instructions on arrival.

## Reliability and verification

Keep an idempotency key across an unconfirmed write; do not retry with a new key to hide
uncertainty. Updates need the expected revision; a conflict means read the current state
before another decision. Errors never echo server bodies, credentials, URLs or exceptions.
Tool results are bounded; malformed/unknown methods and arguments fail closed. Only valid
MCP messages go to stdout. Private screenshots use the separate native viewer, not MCP.

`python3 -m unittest deploy/services/test_mcp.py -q` exercises synthetic protocol,
provenance-independent data handling, field bounds and private-profile denials. The full
repository gate includes these tests. No paid call or production account is needed.
