# Foundation engineering report

Date: 2026-11-13. Scope: the approved Phase 0 architecture/documentation foundation and
Phase 1 authenticated hub-to-worker diagnostic system. The foundation is implemented
and locally validated. No future product phase was silently activated, no paid model
calls were made, and no production deployment or remote mutation was performed.

## Implemented behavior

The Rust hub starts on a validated loopback/private bind and exposes an authenticated
owner API/CLI. It persists task requests, operation identity, idempotency keys, assignments,
state transitions, results, and failures in SQLite. A separately authenticated notebook
worker opens an outbound WebSocket, registers its configured identity and diagnostic
capability, receives a leased assignment, and waits for accepted dispatch authorization.
It journals execution intent, runs the typed side-effect-free diagnostic, journals the
result, and reports completion. The hub validates lease ownership, state, operation
identity, output bounds, and diagnostic acceptance before persisting completion.

The owner can create/list/show tasks, inspect bounded history, list workers, and
pause/resume/cancel eligible tasks. Creation keys and exact repeated results are
idempotent. Expired leases and deadlines reconcile through bounded service operations;
stale reports cannot revive cancelled/paused work or replace completed evidence. Hub
and worker databases are independent and survive reopen. A stable operation identity
supports cached diagnostic results across reconnect/restart. There is no promise of
exactly-once external side effects.

Dispatch reacts to task/control/completion notifications, with a 5-second durable-state
fallback and bounded lease recovery. The production listener admits at most 32 TCP
sockets before HTTP headers/authentication and closes inactive connections after 30
seconds of no actual I/O. Socket admission remains held through WebSocket upgrades.
HTTP requests are capped at 32 in flight; WebSocket connections use a separate configured
cap (eight in the example). Incoming HTTP bodies and WebSocket frames/messages are
limited to 16 KiB. Paginated response record counts and individual payload sizes are
bounded; a complete HTTP page may exceed 16 KiB.

## Architecture and created files

The workspace contains nine focused packages:

- `zaturo-core` in `crates/core`: domain records, task transitions, limits, and acceptance.
- `zaturo-protocol` in `crates/protocol`: strict version-one JSON messages.
- `zaturo-db` in `crates/db`: hub SQLite repositories/migration and separate worker journal.
- `zaturo-services` in `crates/services`: transactional task/worker/lifecycle use cases.
- `zaturo-security` in `crates/security`: credential digests, secure-origin/private-bind
  validation, and a deny-by-default tool policy independent of future models.
- `zaturo-tools` in `crates/tools`: registered `diagnostic.v1` implementation.
- `zaturo-hub` in `apps/zaturo-hub`: CLI, authenticated HTTP/WebSocket composition,
  bounded listener, notification-driven dispatch, and recovery wiring.
- `zaturo-worker` in `apps/zaturo-worker`: outbound transport, journal use, and execution.
- `xtask` in `xtask`: the common local/CI quality gate and repository-contract checks.

The root foundation includes `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`,
`architecture.toml`, `.cargo/config.toml`, `.gitignore`, `.env.example`, `AGENTS.md`,
`CLAUDE.md`, and `README.md`. Each existing crate has focused agent instructions.
`config/` contains strict hub/worker examples and the component/feature registry;
`prompts/` declares an empty executable-prompt registry. `.github/workflows/check.yml`
invokes the common quality command. `tests/benchmark.py` and its README provide the
reproducible local process/load measurement. `deploy/oracle/` and `deploy/notebook/`
contain reviewed example service units and installation profiles, not deployed services.

The documentation includes all requested topic contracts and development recipes, plus
[migration inventory](MIGRATIONS.md), [direct dependency rationale](DEPENDENCIES.md),
this report, and the [recorded benchmark artifact](benchmark-local.json).
[Components](COMPONENTS.md) provides discovery paths rather than competing definitions.

The accepted [decisions](DECISIONS.md) select separate coordination/execution planes,
SQLite WAL with transactional state/events and worker journaling, minimum coherent
crates, private authenticated access, Rust 2024 on pinned verified stable Rust 1.99.0,
and native systemd examples. The worker's hub dependency is permitted only for integration
tests; production dependency boundaries remain checked. Dual MIT OR Apache-2.0 is a
recommendation pending owner adoption, not an assumed license grant.

## Validation actually performed

`cargo xtask check --locked` passed. Its formatting, clippy warnings-as-errors,
architecture/documentation/registry checks, workspace tests, and rustdoc stages completed;
55 tests passed. The tests cover task transitions, transactional persistence, migrations,
claims/leases, bounded retry/deadline behavior, owner controls, capability/authentication
failures, strict protocol/configuration, redaction, worker journal recovery, listener
capacity/idle handling, and actual local hub/worker transport using temporary state.
Tests also exercise invalid repository structures so contract checks are not nominal.

An initial restricted sandbox run could not bind local test sockets. The same required
local-network tests subsequently ran with the necessary local-listener permissions and
passed; that environment limitation is resolved for the reported validation.

`cargo build --release --locked -p zaturo-hub -p zaturo-worker` passed. The release
binaries were exercised as separate local processes by:

```bash
python3 tests/benchmark.py --tasks 100 --concurrency 4 --output docs/benchmark-local.json
```

The benchmark completed and accepted all 100 diagnostic results, then restarted the hub,
verified completed task metadata and five events for a task, and observed worker
reconnection. It uses synthetic credentials and temporary separate SQLite databases.
No production account or external network service was needed.

The systemd unit inspection reached only the expected missing installed binaries at
`/usr/local/bin/zaturo-hub` and `/usr/local/bin/zaturo-worker`; those services were not
installed. Do not treat the examples as a successfully deployed/started systemd service.
The GitHub Actions workflow was created, but no remote CI run is claimed.

## Actual resource observations

The [JSON evidence](benchmark-local.json) records a release run on x86_64 Linux kernel
`7.2.15-76070215-generic`, Rust 1.99.0, 16 logical CPUs, and 33,337,683,968 bytes of host
RAM. This was a local development host, not the Oracle VPS or an established notebook
capacity test. The artifact includes executable SHA-256 hashes and database/WAL sizes.

- Workload: 100 diagnostics, four concurrent HTTP submissions, one executing worker task.
- Diagnostic load duration: 9.103539 seconds; total benchmark duration: 11.34064 seconds.
- Idle hub RSS after registration/warm-up: 8,708,096 bytes, or 8.3046875 MiB.
- Observed peak hub RSS/kernel high-water mark: 9,551,872 bytes, or 9.109375 MiB.
- Sampling: 50 ms interval, 207 hub samples during the main measurement.
- Constraint: `RLIMIT_AS=314572800` bytes (300 MiB virtual address space), with
  `MALLOC_ARENA_MAX=2`. This is not a cgroup/RSS cap; the worker was outside the limit.

The observed RSS is below the design targets for this synthetic run. It does not prove
Oracle production capacity, sustained unattended behavior, future LLM/browser/video
resource use, or indefinite database growth. A target-machine workload and a separately
measured systemd/cgroup memory cap remain necessary before operational guarantees.

## Implemented security boundaries and limitations

Owner and worker tokens are distinct, validated, held as digests at the hub security
boundary, and compared using constant-time digest comparison. Debug output redacts the
credential wrapper. Worker registration is bound to the configured identity; owner and
worker endpoint roles are separate. The worker permits only the typed read-only
`diagnostic.v1` capability. Other risk classes and unknown tools fail closed. There is
no unrestricted shell, credential broker, browser, provider call, or account integration.

Shared endpoint validation requires HTTPS for remote origins and permits HTTP only for
literal loopback. It rejects embedded credentials, arbitrary paths, queries, and
fragments. Hub configuration rejects public/unspecified bind addresses. VPN/firewall ACLs
and private TLS termination remain installation responsibilities; no TLS server, VPN,
certificate provisioning, or public deployment was installed here. A private address
range alone is not proof of authorization.

Known foundation limits are one configured worker, one active worker task, a total
creation-to-completion task deadline of at most 300 seconds, and at most three attempts.
Pause/resume cannot reset that budget. Service tokens are static bootstrap credentials;
rotation requires operator coordination and there is no session/device enrollment UI.
SQLite and worker journal contents are not encrypted. Automated audit archival, artifact
quotas, long-running checkpoints, process-group cancellation for future tools, and
external-effect reconciliation are future work. The synthetic diagnostic cannot attest
a compromised worker's honesty. The API/CLI is the available client; no dashboard exists.

A private ignored `.env` was prepared locally with owner-only permissions and generated
service tokens. Runtime binaries do not load it automatically. Deployment-workstation
SSH variables are separate preparation metadata and are not consumed by the applications.
No private key contents, real tokens, or owner certificates are committed in examples.
Other local projects and certificate material remain outside this implementation's
modification scope. Follow-up Oracle preparation remains pending working SSH access at
report time; no VPS deletion, modification, installation, or deployment is claimed.

## Intentionally deferred and unresolved decisions

AI orchestration and model routing, verified eligible OpenCode Go transport, computer
use/human takeover, encrypted vault/broker/OAuth, GitHub/video/price/Gmail adapters,
recurring/overnight scheduling, notifications, graphical/mobile clients, and advanced
multi-worker autonomy remain planned. Their contracts, dependencies, threats, and
acceptance gates are documented and marked consistently in [roadmap](ROADMAP.md).

Owner decisions remain license adoption, private VPN/TLS topology and provisioning,
provider workload/privacy authorization, notebook hardware/display capabilities, vault
unlock/recovery policy, external-effect approval scopes, retention/backup policy,
overnight windows/budgets, and client priorities. See [open questions](OPEN_QUESTIONS.md).

## Run the system locally

From the repository root, use one consistent set of distinct generated credentials
across the terminals. If using the private local `.env`, explicitly export it in each
relevant shell; never print or commit its contents:

```bash
set -a
source .env
set +a
cargo run -p zaturo-hub -- serve
```

In another appropriately configured shell:

```bash
cargo run -p zaturo-worker -- connect
```

In an owner-configured shell:

```bash
cargo run -p zaturo-hub -- task create --kind diagnostic --target local-worker \
  --idempotency-key foundation-diagnostic
cargo run -p zaturo-hub -- task list
cargo run -p zaturo-hub -- task show TASK_ID
cargo run -p zaturo-hub -- workers list
```

Replace `TASK_ID` with the returned UUID. Use `task pause`, `task resume`, or `task cancel`
with a nonterminal eligible task. Both binaries accept `--config PATH`; defaults use the
committed local examples. The [README](../README.md), [deployment](DEPLOYMENT.md), and
[testing](TESTING.md) contain detailed setup and reproducibility boundaries.

The next recommended product phase is Phase 2: a bounded orchestrator and fake-model
acceptance tests, followed by a provider adapter only after official endpoint, workload,
and data-policy verification. Production deployment and the requested Oracle preparation
are separate operational work; they are not evidence that future product features exist.
