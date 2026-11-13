# Notebook installation profile

These examples cover the diagnostic worker only. Read
[deployment](../../docs/DEPLOYMENT.md) before any authorized installation.

Use Debian Stable, or a tested Ubuntu LTS build. Prepare the dedicated `zaturo-worker`
account. Install the reviewed release binary as `/usr/local/bin/zaturo-worker` and copy
`config/worker.example.toml` to `/etc/zaturo/worker.toml`. Set the remote hub's private
HTTPS origin, retain the registered `worker_id`, and set absolute paths beneath
`/var/lib/zaturo-worker` for the worker database, workspaces, and artifacts.

`/etc/zaturo/worker.env` needs only the matching `ZATURO_WORKER_TOKEN` and optional
`RUST_LOG=info`; never copy the owner's hub credential or external account keys into it.
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
