# Live hub and local worker

The owner authorized service activation, Phase 2 implementation, paid DeepSeek calls,
and publication of code after validation on 2026-11-13. The Oracle hub and this PC's
worker run as systemd services, independently of the coding-agent session. Actual task
and validation evidence belongs to the [Phase 2 report](PHASE2_REPORT.md).

## Private network path

The VPS hub binds `127.0.0.1:7878`. The PC opens an outbound SSH connection on port 22
and forwards `127.0.0.1:17878` to that remote loopback address. HTTP and WebSocket
messages stay inside the encrypted SSH path between computers. No home-router port
forwarding, public hub listener, new web firewall allowance, VPN account, or TLS proxy
is required for this topology. Remote direct plaintext HTTP remains rejected by clients.

The VPS firewall retains deny-incoming policy and only the existing SSH allowance.
Host-key verification stays enabled. A changed host key requires independent verification.
The owner-supplied key pair remains in ignored `.secrets/ssh/`; neither key contents
nor the actual server address appears in public configuration templates.

## Installed service boundaries

The hub uses the dedicated unprivileged `zaturo-hub` account:

- Binary: `/usr/local/bin/zaturo-hub`.
- Unit: `/etc/systemd/system/zaturo-hub.service`, enabled at boot.
- Configuration: `/etc/zaturo/hub.toml`, `root:zaturo-hub`, mode `0640`, under a `0750` directory.
- Service-only environment: `/etc/zaturo/hub.env`, root-owned mode `0600`.
- State: `/var/lib/zaturo-hub/hub.db`, under a hub-owned `0700` directory.

The PC uses owner-account services because local administrative sudo is unavailable:

- `zaturo-hub-tunnel.service`: [template](../deploy/notebook/zaturo-hub-tunnel.user.service).
- `zaturo-worker.service`: [template](../deploy/notebook/zaturo-worker.user.service).
- Binary: `~/.local/lib/zaturobot/zaturo-worker`.
- Private configuration/environment: `~/.config/zaturobot/`, mode `0700`; files `0600`.
- State, workspace, and artifact roots: `~/.local/share/zaturobot/`.

User lingering is enabled so the user manager can start at boot and outlive interactive
logins. The worker reconnects after temporary tunnel/network loss; systemd restarts either
service after process failure. They are not child processes of this chat. No dedicated
desktop session or separate worker Unix user is claimed; those isolation requirements
must be addressed before future computer-use tools are activated.

## Credentials and the selected model

The root project `.env` is private and ignored. Deployment copies use separate roles:
`owner.env` for the owner CLI, `worker.env` for device authentication and the provider key,
and `tunnel.env` for SSH connection metadata. The hub environment contains only owner
and worker service tokens plus logging settings. The OpenCode Go key stays on this PC;
it is never installed on the hub or embedded in a binary, unit, prompt, example, or commit.

Only `deepseek-v4.1-flash` is allowed. The active worker model profile explicitly enables
the Global processing route and public-coding data category. It denies private/personal
data and training-enabled routes. An assisted task requires explicit
`--data-policy public-coding`; the flag is an owner declaration, never a model grant.
Read [provider verification](OPENCODE_GO_RESEARCH.md) before changing that policy.

The active profile uses one concurrent request, a 30-second request timeout, 8 KiB context,
16 KiB response cap, and at most 2,048 output tokens. Durable admission ceilings are
16,384 tokens/request, 65,536/task, 131,072/UTC day, and 1,048,576/UTC month, with at most
four calls/task. The orchestrator has two model steps and one permitted diagnostic tool.
Retries consume separate reservations. Unknown usage remains charged conservatively;
neither a timeout nor restart authorizes replay. Token admission is not a dollar cap.

## Operate from the PC

```bash
systemctl --user status zaturo-hub-tunnel.service zaturo-worker.service
journalctl --user -u zaturo-worker.service -n 30 --no-pager
systemctl --user restart zaturo-hub-tunnel.service zaturo-worker.service
loginctl show-user "$USER" -p Linger
```

For owner commands, export only the owner service token from its private environment
file. The ignored `config/hub.remote.local.toml` is a CLI profile pointing `client_url`
at `http://127.0.0.1:17878`; create it from the hub example when configuring another PC.
Do not start a second hub using that client profile.

```bash
set -a
source "$HOME/.config/zaturobot/owner.env"
set +a
cargo run -p zaturo-hub -- --config config/hub.remote.local.toml workers list
cargo run -p zaturo-hub -- --config config/hub.remote.local.toml task create \
  --kind assisted_diagnostic --target local-worker --data-policy public-coding \
  --request 'Run a read-only diagnostic of this public Rust development worker.'
```

On the VPS, inspect the hub using `sudo systemctl status zaturo-hub` and structured
journald logs. Never print an environment file or authenticated headers while debugging.

## Updates and recovery

Build release binaries on the PC, never on the constrained VPS. The 0.2.0 update used
consistent SQLite backups and retained 0.1.0 binaries before changing installed files.
The worker database migrates from schema 2 to 3; the hub schema remains 1 with additional
backward-readable fields in old task records. Protocol 2 requires a coordinated hub and
worker update. Older binaries may not understand newly written assisted tasks or the
new worker schema, so rollback requires restoring the matching database snapshot.

Stop the worker before its binary/journal update; stop the hub before replacing its
binary, then restart hub and worker and verify an authenticated task. Private snapshots
are retained in ignored deployment state and on the VPS for operator recovery. Do not
replace live SQLite files while either process is using them. See [migrations](MIGRATIONS.md).

The earlier PDF retirement remains recorded in the historical
[reuse report](../deploy/oracle/REUSE_VPS.md). Its encrypted recovery archive and all local
PDF files/certificates remain preserved. Desktop control, GitHub account automation,
email/video integrations, and autonomous scheduling are still future phases.
