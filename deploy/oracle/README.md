# Hub installation profile

This profile describes service installation. The separate
[Oracle reuse report](REUSE_VPS.md) is the canonical record of owner-authorized host
preparation; preparing directories and staging a unit do not install or start the hub.
Read [deployment](../../docs/DEPLOYMENT.md) and [security](../../docs/SECURITY.md) first.

Use an unprivileged `zaturo-hub` account with a non-login shell and no interactive
password. On a separately authorized installation, place the reviewed release binary at
`/usr/local/bin/zaturo-hub`. The configuration directory `/etc/zaturo` should be
`root:zaturo-hub` mode `0750`; `hub.toml` should be `root:zaturo-hub` mode `0640`.
The service user must be able to traverse the directory and read its nonsecret config.
A root-only `0700` directory or `0600` config would block this service user.
Place a copy of `config/hub.example.toml` at `/etc/zaturo/hub.toml`; keep `bind` loopback unless a private VPN address is deliberately
chosen. Set `database = "/var/lib/zaturo-hub/hub.db"`. Set `client_url` to the private
HTTPS origin that the owner CLI will use, or leave loopback for local administration.
The URL is not the bind address; TLS is terminated by the reviewed private proxy.

Create `/etc/zaturo/hub.env` with locally generated distinct `ZATURO_OWNER_TOKEN` and
`ZATURO_WORKER_TOKEN`, plus `RUST_LOG=info`. Protect the directory/files and never commit
actual tokens. Keep this environment file `root:root` mode `0600`: the system systemd
manager reads it before dropping to the service user. Do not weaken secret-file access
to fix a configuration-directory permission error. The service environment is not a
vault for external account credentials. `/var/lib/zaturo-hub` is owned by the hub user
and mode `0700`; `/opt/zaturobot`, if used for prepared release storage, is root-owned
and mode `0755`. A staged unit under `/etc/zaturo` is not an enabled systemd service.
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
