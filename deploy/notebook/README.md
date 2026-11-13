# Notebook installation profile

These examples cover the direct and bounded model-assisted diagnostic worker. Read
[deployment](../../docs/DEPLOYMENT.md) before any authorized installation.

Use Debian Stable, or a tested Ubuntu LTS build. Prepare the dedicated `zaturo-worker`
account. Install the reviewed release binary as `/usr/local/bin/zaturo-worker` and copy
`config/worker.example.toml` to `/etc/zaturo/worker.toml`. Set the remote hub's private
HTTPS origin, retain the registered `worker_id`, and set absolute paths beneath
`/var/lib/zaturo-worker` for the worker database, workspaces, and artifacts.

`/etc/zaturo/worker.env` needs the matching `ZATURO_WORKER_TOKEN` and optional
`RUST_LOG=info`. The explicitly enabled model route additionally needs its provider key
on this worker only; never copy that key to the hub or copy the owner's hub credential
into the worker environment.
Protect directory/files with restrictive permissions. The worker initiates outbound
WSS; no router port forwarding or inbound notebook listener is required.

Review [zaturo-worker.service](zaturo-worker.service). It permits writing only its state
directory and starts without a desktop. CPU/RAM containment is configured in systemd,
not as unsupported runtime TOML fields. Future builds/video/desktop will need a separate
reviewed profile; this 512 MB example is deliberately sized for diagnostic-only work.

After authorized installation, activate the unit using `systemctl daemon-reload` and
`systemctl enable --now zaturo-worker`. Inspect `journalctl -u zaturo-worker`, register
visibility from the owner CLI, and complete one diagnostic. Stop/restart with the same
worker database to preserve operation evidence. Do not delete the journal as a generic
reconnect fix. No GPU, Xvfb, browser, or desktop package is required in this phase.

## Current PC user-service profile

The owner-authorized current-PC installation uses systemd user services and the existing
verified SSH identity. The hub stays loopback-bound on the VPS; the outbound SSH tunnel
forwards it to `127.0.0.1:17878` locally. No inbound router or public hub port is required.
The current Unix user is not the future isolated computer-use user/session.

Public unit templates are `zaturo-hub-tunnel.user.service` and
`zaturo-worker.user.service`. Private configuration lives under
`~/.config/zaturobot/` (directory mode `0700`): `worker.toml`, optional `models.toml`,
`worker.env`, and `tunnel.env`. Keep environment files `0600`; never commit their values.
The API provider key belongs only to the worker-local environment. The SSH private-key
path references the owner's ignored local copy, never a private key embedded in a unit.

The reviewed worker binary is installed at `~/.local/lib/zaturobot/zaturo-worker`.
Persistent database/workspace/artifact paths are under `~/.local/share/zaturobot/`.
User unit files under `~/.config/systemd/user/` use mode `0644` and contain no secrets.
Enable/restart with `systemctl --user`; inspect `journalctl --user -u zaturo-worker` and
`-u zaturo-hub-tunnel`. User lingering must be verified for operation after logout.
Closing a terminal is not a reason to delete the worker journal or repeat an uncertain
paid request. Actual deployed state and smoke evidence are recorded by the operational
runbook; templates alone do not establish that a service is healthy.
