# Procurali zero-downtime pull auto-deploy

This directory contains the isolated zero-downtime deployment helpers for the Procurali
commerce hub, worker, Telegram daemon, and static site builder on the Linux VPS.

## How CI-gated auto-deploy works

1. `.github/workflows/check.yml` runs the `check` (`cargo xtask check --locked`) and
   `android` jobs on every push and pull request.
2. Only after **both** `check` and `android` succeed on a `push` to `main`, the
   `release-bundle` job compiles Linux release binaries (`zaturo-hub`, `zaturo-worker`),
   packages the pure-Node static site builder, writes `manifest.json` + SHA-256 digests,
   and publishes an immutable GitHub Release (`procurali-<shortsha>`).
   No VPS IP addresses, hostnames, or SSH keys are stored on GitHub.
3. On the VPS, `procurali-deploy-agent.timer` runs `procurali_deploy_agent.py` periodically:
   - Fetches the latest `procurali-*` release metadata and verifies independently via the
     GitHub Check Runs API that `check` and `android` both finished with `success` on that
     exact `commit_sha`.
   - Verifies bundle and binary SHA-256 digests cleanly with safe tar extraction.
   - Backs up SQLite WAL (`hub.db`) via `sqlite3 .backup` before starting new code.
   - Starts the standby Hub slot (`a` on `127.0.0.1:17881` <-> `b` on `127.0.0.1:17882`),
     verifies `/healthz`, validates Caddy syntax, reloads Caddy (`127.0.0.1:7878`) with zero
     dropped HTTP connections, drains the previous slot, and rolls back automatically on failure.
   - `procurali_site_sync.py` publishes static frontend builds into versioned release directories
     and performs an atomic POSIX symlink swap (`mv -Tf`) for `/srv/procurali/site`.

See [deployment documentation](../../docs/DEPLOYMENT.md) and
[repository tests](../../xtask/src/main.rs).
