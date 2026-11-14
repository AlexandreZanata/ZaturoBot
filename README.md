# ZaturoBot

A single-owner Linux automation foundation: a lightweight coordination hub and an
outbound Linux workers. It persists task history in SQLite, reconciles durable leases
and worker journals, and supports a direct diagnostic plus a bounded model-assisted
public coding diagnostic. The direct diagnostic needs no AI API key; the assisted route
uses the owner-selected OpenCode Go model. Neither needs Docker, a desktop, or a GPU.

The code and engineering documentation are English. Future owner interfaces will default
to Brazilian Portuguese and support English. Start with [AGENTS.md](AGENTS.md) when
changing code and the [documentation map](docs/README.md) for detailed contracts.

## Run locally

Install the pinned Rust toolchain and a C compiler for bundled SQLite. From the repository
root, generate two distinct local service tokens in a private shell. These commands set
only this shell's environment; the application does not load `.env` automatically.

```bash
export ZATURO_OWNER_TOKEN="$(openssl rand -hex 32)"
export ZATURO_WORKER_TOKEN="$(openssl rand -hex 32)"
cargo build --workspace
```

Run the hub in that shell:

```bash
cargo run -p zaturo-hub -- serve
```

In a second terminal, set the same `ZATURO_WORKER_TOKEN` securely and start the worker:

```bash
cargo run -p zaturo-worker -- connect
```

In a third terminal, set the same `ZATURO_OWNER_TOKEN` securely and create a task:

```bash
cargo run -p zaturo-hub -- task create --kind diagnostic --target local-worker \
  --idempotency-key first-diagnostic
cargo run -p zaturo-hub -- task list
cargo run -p zaturo-hub -- task show TASK_ID
cargo run -p zaturo-hub -- workers list
```

Replace `TASK_ID` with the UUID returned by creation/list. `show` returns the task plus
the first bounded event page; use the API to page through longer histories. Reusing the
same idempotency key with the same request returns the same task. Changing its payload
while reusing the key is a conflict. Default data files are `data/hub.db` and
`data/worker.db`; keep them separate.

Owner controls are `task pause TASK_ID`, `task resume TASK_ID`, and `task cancel TASK_ID`.
The diagnostic is very short, so it may already be terminal before a control request.
A terminal task cannot resume. Restart the hub/worker against their original databases
to exercise persisted recovery; only the read-only diagnostic is safe to repeat.

Both binaries accept a global `--config PATH`. Default examples are
[hub](config/hub.example.toml) and [worker](config/worker.example.toml).
The hub binds `127.0.0.1:7878`. Remote worker/client origins require HTTPS; deployment
uses VPN/private reachability plus reviewed TLS termination or a secure local tunnel.
Local settings are not a public-internet deployment recipe.

## Model-assisted coding diagnostic

The only configured model is `deepseek-v4.1-flash`. Read
[provider policy](docs/LLM_PROVIDERS.md) and the disabled
[model example](config/models.example.toml) before activation. The worker receives a
protected local model-config path and API key; the hub never receives that key.
Explicitly authorize public-coding disclosure and Global processing. The model can
request only the existing read-only diagnostic, then produce a bounded summary.

```bash
cargo run -p zaturo-hub -- task create --kind assisted_diagnostic \
  --target local-worker --data-policy public-coding \
  --request "Inspect the public Rust coding environment with the approved diagnostic."
```

This requires a worker advertising `agent.diagnostic.v1` through a validated real
provider configuration. Model-disabled workers retain the direct diagnostic only.
Fake models are test fixtures, never an undisclosed production fallback. Budget
reservations survive restart and uncertain paid requests stop for reconciliation.

The installed private services and owner CLI profile are documented in
[the live deployment runbook](docs/LIVE_DEPLOYMENT.md); current execution evidence is in
[the Phase 2 report](docs/PHASE2_REPORT.md). The hub remains reachable through the
verified SSH tunnel, with operational keys and tokens excluded from the source artifact.

## Files between computers

Version 0.3.0 adds encrypted resumable files through the VPS with separate credentials
per device. Both PCs initiate pinned SSH connections; no home-router port is needed.
Only ciphertext is stored on the relay. The worker verifies and saves incoming files
without opening/executing them or sending them to a model.

Use [Ubuntu/Debian enrollment](deploy/device/README.md) with the private owner-issued
bundle. Installed profiles provide a `~/.local/bin/zaturo` wrapper:

```bash
~/.local/bin/zaturo files peers
~/.local/bin/zaturo files send /absolute/path/report.pdf --to work-pc
~/.local/bin/zaturo files list
~/.local/bin/zaturo task diagnostic
```

See [file contracts and recovery](docs/FILE_TRANSFERS.md) and
[measured VPS verification](docs/FILE_TRANSFER_REPORT.md). The physical work PC still
runs its protected enrollment handoff; a simulated second profile is not that installation.

## Validate

```bash
cargo xtask check --locked
cargo xtask arch
cargo xtask docs
cargo xtask secrets
```

See [testing](docs/TESTING.md), [deployment](docs/DEPLOYMENT.md),
[security](docs/SECURITY.md), and [resource limits](docs/RESOURCE_LIMITS.md).
The current quality gate passed 107 tests; Phase 2 live execution and restart evidence
are recorded in [its report](docs/PHASE2_REPORT.md). A recorded 100-task local benchmark
observed 8.30 MiB idle hub RSS and 9.11 MiB peak RSS; this is development-machine
evidence, not an Oracle capacity guarantee. Read the
[engineering report](docs/ENGINEERING_REPORT.md) for commands, environment, constraints,
and known limitations. The production listener bounds TCP admission and idle time
before authentication; remote access still requires a private secure network path.

## Scope and license

The owner authorized Phases 0–2, private hub/worker activation, encrypted cross-network device files and bounded multiple
diagnostic workers, and GitHub publication
after validation. Computer use, encrypted vault, personal service adapters, recurring
scheduling, and graphical/mobile clients remain [planned](docs/ROADMAP.md). This phase
does not add arbitrary project editing, desktop control, or an unrestricted shell.
Operational keys, tokens, databases, and backups stay ignored and untracked; the
publication guard checks visible work, staging, and reachable history before push.

Dual MIT OR Apache-2.0 is recommended; license adoption remains an owner decision in
[ADR-0007](docs/DECISIONS.md). No license grant is assumed from repository visibility.
