# Local and native deployment

The initial implementation is development-only until the owner chooses and reviews a
private installation. No VPS access, deployment, router changes, or package installation
was performed during this foundation pass. Local commands are in the [README](../README.md).

## Oracle hub

Build `zaturo-hub` in release mode on a compatible Linux build machine and copy the
binary after explicit deployment authorization. Do not compile external projects on
the 1 GB VPS. Create a dedicated unprivileged `zaturo-hub` account, an owner-protected
configuration directory, and a writable state directory. Use
[hub service](../deploy/oracle/zaturo-hub.service) as a reviewed starting point.

The [hub installation profile](../deploy/oracle/README.md) identifies exact paths and
post-install checks. Keep the process loopback-bound behind a private VPN/TLS path. Tailscale is the initial
VPN preference; Headscale adds operational cost and needs measured justification.
Remote worker/client URLs must use HTTPS/WSS or the explicitly permitted private tunnel.
Do not open a public unauthenticated control API. A proxy must support WebSocket upgrade,
propagate connection closure, and avoid logging Authorization headers.

Use separate random owner and worker tokens, generated locally, not committed examples.
Protect the environment file with mode 0600 and directory with 0700. Tokens are service
credentials only; never put Gmail/GitHub/provider keys on the hub. Rotate a token by
updating both relevant endpoints and restarting them; an API rotation workflow is future work.

## Debian notebook

Debian Stable with a dedicated `zaturo-worker` user is preferred; Ubuntu LTS is supported
as an alternative build/run target. The diagnostic needs no display or GPU. Use
[worker service](../deploy/notebook/zaturo-worker.service), a protected token file, local
worker database, workspace root, and artifact root. The [notebook installation profile](../deploy/notebook/README.md) maps paths and limits.
The notebook only opens outbound connections. Keep its identity and approved capabilities aligned with hub configuration.

Future desktop work uses a separate Xvfb/Openbox session, optionally Xfce. Do not install
or start it for the foundation. GPU/video compatibility remains unverified.

## Persistence, recovery, and operations

SQLite WAL files belong with each database. Back up through SQLite's consistent backup
mechanism or stop the service before copying database and sidecars. Never synchronize
hub and worker database files. Before upgrades, back up both and read
[migrations](MIGRATIONS.md); restoring an older binary after schema changes may require
restoring its backup. Check logs and authenticated task history after restart. Never
manually mark an uncertain operation completed just to clear the queue.

systemd caps are examples requiring testing on the target machine. Logging retention,
disk monitoring, TLS certificates, VPN ACLs, backup encryption, and token rotation are
operator responsibilities. Review [security](SECURITY.md) and [limits](RESOURCE_LIMITS.md).
