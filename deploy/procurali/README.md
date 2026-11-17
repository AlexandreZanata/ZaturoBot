# Procurali zero-downtime pull auto-deploy

This directory contains the isolated zero-downtime deployment helpers for the Procurali
commerce hub, worker, Telegram daemon, and static site builder on the Linux VPS.

## How CI-gated auto-deploy works

1. `.github/workflows/check.yml` runs the `check` (`cargo xtask check --locked`) and
   `android` jobs on every push and pull request.
2. Only after **both** `check` and `android` succeed on a `push` to `main`, the
   `release-bundle` job compiles Linux release binaries (`zaturo-hub`, `zaturo-worker`),
   packages the Next.js storefront as a self-contained site builder
   ([package_site_builder.py](package_site_builder.py): sources, pinned `node_modules`
   without symlinks or musl/wasm/sharp extras, and `runtime/node` from CI's Node 24),
   smoke-tests it exactly as the VPS calls it, writes `manifest.json` + SHA-256 digests,
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

## Site builder contract

`procurali_site_sync.py` runs, for every new catalog ETag or changed builder sources:

```text
<site-builder>/runtime/node <site-builder>/src/build.mjs --api-snapshot <catalog.json> --out-dir <release>
```

`src/build.mjs` runs `scripts/prepare.mjs` and `next build` into a temporary distDir, then copies
the static export into `<release>`; the release goes live only after `index.html`,
`ofertas/index.html`, `api/*.json`, `sitemap.xml` and `_next/static/` are verified. A failed build
keeps the current site live and retries with exponential backoff (up to 15 minutes).
Environment overrides: `PROCURALI_SITE_NODE` (Node executable), `PROCURALI_SITE_BUILD_TIMEOUT_SECS`
(default 900) and `PROCURALI_BUILD_CPUS` (Next.js static-generation workers, default 2).

The unit runs with `ProtectSystem=strict`/`ProtectHome=true`, so the release under `/opt` is
read-only: the builder copies it once per release into
`/var/lib/procurali-site-sync/builder/work/` (older copies are removed) and keeps `TMPDIR` and
`HOME` beside it. The Hub token and URL are removed from the builder environment because the
site sync already fetched the snapshot. `next build` needs more than the original
256M/512M/50% CPU budget; install [the drop-in](procurali-site-sync.nextjs.conf) as
`/etc/systemd/system/procurali-site-sync.service.d/10-nextjs-build.conf`
(`MemoryHigh=1536M`, `MemoryMax=2G`, `CPUQuota=200%`).
The builder digest covers the builder sources but not generated output, so each release that
changes the storefront forces one rebuild. The deploy agent keeps the three newest releases.

See [deployment documentation](../../docs/DEPLOYMENT.md) and
[repository tests](../../xtask/src/main.rs).
