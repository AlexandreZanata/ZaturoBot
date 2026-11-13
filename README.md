# ZaturoBot

A single-owner Linux automation foundation: a lightweight coordination hub and an
outbound notebook worker. The current implementation executes one safe diagnostic,
persists task history in SQLite, and reconciles leases and worker journal state.
It needs no AI API key, Docker, desktop, GPU, or external account.

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

## Validate

```bash
cargo xtask check --locked
cargo xtask arch
cargo xtask docs
```

See [testing](docs/TESTING.md), [deployment](docs/DEPLOYMENT.md),
[security](docs/SECURITY.md), and [resource limits](docs/RESOURCE_LIMITS.md).
The verified local quality gate passed 55 tests. A recorded 100-task local benchmark
observed 8.30 MiB idle hub RSS and 9.11 MiB peak RSS; this is development-machine
evidence, not an Oracle capacity guarantee. Read the
[engineering report](docs/ENGINEERING_REPORT.md) for commands, environment, constraints,
and known limitations. The production listener bounds TCP admission and idle time
before authentication; remote access still requires a private secure network path.

## Scope and license

Phases 0/1 are the approved scope. AI orchestration, computer use, encrypted vault,
GitHub/video/price/Gmail adapters, recurring scheduling, and graphical/mobile clients
are [planned](docs/ROADMAP.md). The foundation does not claim those features work.

Dual MIT OR Apache-2.0 is recommended; license adoption remains an owner decision in
[ADR-0007](docs/DECISIONS.md). No license grant is assumed from repository visibility.
