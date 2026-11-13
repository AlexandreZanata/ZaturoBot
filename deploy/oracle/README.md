# Hub installation profile

These are reviewable installation examples, not an executed deployment. Read
[deployment](../../docs/DEPLOYMENT.md) and [security](../../docs/SECURITY.md) first.

Prepare an unprivileged `zaturo-hub` account and install the reviewed release binary as
`/usr/local/bin/zaturo-hub`. Place a copy of `config/hub.example.toml` at
`/etc/zaturo/hub.toml`; keep `bind` loopback unless a private VPN address is deliberately
chosen. Set `database = "/var/lib/zaturo-hub/hub.db"`. Set `client_url` to the private
HTTPS origin that the owner CLI will use, or leave loopback for local administration.
The URL is not the bind address; TLS is terminated by the reviewed private proxy.

Create `/etc/zaturo/hub.env` with locally generated distinct `ZATURO_OWNER_TOKEN` and
`ZATURO_WORKER_TOKEN`, plus `RUST_LOG=info`. Protect the directory/files and never commit
actual tokens. The service environment is not a vault for external account credentials.
The example unit's `StateDirectory` creates writable service state; its `User` account
must already exist. Install [zaturo-hub.service](zaturo-hub.service) only after reviewing
paths, resource caps, VPN ACLs, proxy WebSocket support, and idle/connection limits.

After an authorized install, `systemctl daemon-reload` and
`systemctl enable --now zaturo-hub` activate the unit. Check `systemctl status zaturo-hub`,
`journalctl -u zaturo-hub`, loopback `/healthz`, and an authenticated task request.
Configure journald disk/retention quotas at the host level and monitor free storage.
Do not paste authenticated environment output into issue reports.

Build on a compatible machine, not by compiling other projects on the 1 GB hub.
The example `MemoryMax=300M` is containment; target-machine validation is still required.
