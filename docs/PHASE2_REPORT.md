# Phase 2 engineering report

Date: 2026-11-13. Release: 0.2.0. The owner authorized private hub/worker activation,
Phase 2, bounded paid verification using only `deepseek-v4.1-flash`, and GitHub
publication after validation. Keys, credentials, state, and recovery archives remain
outside the source artifact. Later roadmap phases were not activated.

## Implemented behavior

The phase adds provider-independent requests/client/router, a verified bounded OpenCode
Go adapter, and one structured public-coding diagnostic planner. A model can propose the
existing read-only diagnostic once, then return a bounded summary. Strict parsing,
independent policy, typed evidence, lease fencing, deadlines, and acceptance checks
remain between the proposal and completion. Unknown tools, extra fields, early finish,
and repeated tool actions are rejected. No unrestricted shell was added.

Worker schema 3 adds atomic paid-request reservation/usage records and durable assisted
outcomes. Request/task/UTC-day/UTC-month token admission and task-call caps survive
restart. Pending or uncertain requests remain charged; overrun becomes retained debt.
Cached final outcomes can be reported without another provider request. Unresolved paid
attempts stop for reconciliation. Current admission is token/call based, not a dollar cap.

Protocol 2 adds assisted completion and typed fenced failure messages. Old direct task
results remain readable through optional persisted metadata, while incompatible live
peers must be upgraded coherently. Hub schema remains 1; worker schema upgrades 2 to 3.
Old binaries and consistent database snapshots were retained before the live update.

## Tests and repository validation

`CARGO_NET_OFFLINE=true cargo xtask check --locked` passed all **100 tests** with no
failures or ignored tests. Formatting, strict all-target Clippy, rustdoc, architecture,
documentation/configuration/prompt contracts, and the publication guard passed. This
includes 24 `xtask` tests with explicit prompt owner/purpose validation and the five
secret-publication regressions. Both systemd user-service templates also passed
`systemd-analyze --user verify` without diagnostics. Installed hub and worker executable
hashes matched their reviewed 0.2.0 release artifacts.

This is actual local validation. A remote GitHub Actions result is not claimed before
the publication-triggered workflow has run.

The suite includes fake-model and synthetic HTTP tests, budget/UTC-boundary/race tests,
version-2 worker migration, and real local HTTP/WebSocket worker tests. Assisted
transport tests prove one journaled tool/two model steps, denial of a malicious shell
proposal, responsive heartbeats during delayed inference, owner cancellation preserving
uncertain spend, and restart without replaying a reserved paid request. CI uses synthetic
credentials/fixtures only and never calls the owner's provider account.

The publication guard inspects nonignored Git-visible working files, staged blobs,
and reachable historical blobs/commit/tag messages. Known private paths and credential
forms fail without printing values. Ignored `.env` and `.secrets/` are never opened.
Five leakage regressions cover private paths, markers, ignored files, staged/history
leaks, and commit messages. CI fetches reachable history. This heuristic complements
staged-diff review; it is not proof against arbitrary encoded secrets.

The foundation's older 55-test result and recorded development-machine benchmark remain
historical evidence, not the Phase 2 test total or an Oracle load benchmark.

## Verified live execution

At 19:24:40 America/Cuiaba (23:24 UTC), both installed services ran release 0.2.0 with
protocol 2. An authenticated owner request explicitly classified `assisted_diagnostic`
as public coding. Task `2c8134f9-d3e2-4a98-8843-91d0cd4015f0` reached `COMPLETED` through
`CREATED`, `QUEUED`, `ASSIGNED`, `RUNNING`, and `COMPLETED`.

The real provider result identified only `deepseek-v4.1-flash`, two planning steps,
two model calls, and complete confirmed usage: 832 input tokens and 99 output tokens.
The provider reported `cost_microusd = 0` for those requests. This is not a claim that
future requests are free or that no subscription allowance was consumed; upstream Go
allowance is not substituted for monetary cost by the implementation.

The permitted diagnostic reported Linux/x86_64 success without inspecting code files
or modifying the system. Its typed evidence, not the natural-language summary, is the
authoritative acceptance record. Private verification artifacts retain detailed service,
request, and journal evidence without publishing credentials or the real server address.

At 19:28:35 America/Cuiaba (23:28 UTC), live restart checks passed: the hub, worker, and
tunnel returned active; the assisted result and original foundation task persisted;
and a new direct diagnostic completed after restart. The worker database was schema 3,
with exactly two model reservations, both settled, totaling the same 931 charged tokens.
No extra model calls occurred during those restart checks.

## Operational and security evidence

The [live deployment runbook](LIVE_DEPLOYMENT.md) owns current paths, unit names,
private configuration, and operator commands. The VPS hub is a dedicated unprivileged
system service on loopback. The PC's enabled systemd user worker and SSH tunnel use
lingering and reconnect/restart independently of this chat. The encrypted SSH22 path
forwards only a local loopback origin; UFW retains only SSH ingress for IPv4/IPv6.

The provider key was verified absent from the remote hub environment, which contains
only owner/device service tokens and logging configuration. Worker-local private
configuration enables the sole model and explicit Global/public-coding policy. No key,
real token, database, or private recovery backup is tracked by Git. The owner's other
local projects and certificates remain outside modification scope.

After a fresh hub restart, the observed RSS/high-water snapshot was 8,308 KiB with four
threads. This is one small-workload snapshot, not a sustained/load/cgroup benchmark or
a production capacity guarantee. The earlier synthetic local benchmark is separate.

## Known limits and next phase

Only the bounded public-coding diagnostic route is implemented. It does not edit arbitrary
projects, operate a desktop/browser, access a vault or personal accounts, send messages,
upload videos, schedule recurring work, or supply mobile clients. Privacy classification
is an owner assertion, not automatic secret detection. Model summaries remain untrusted.
Hub validation cannot attest a compromised worker's claimed model execution or OS state.

The current worker uses the owner's Unix account and is not the future isolated desktop
session. Persistent encrypted vault storage, broader provider eligibility, monetary caps,
retention policies, and future side-effect tools require their own contracts. The next
roadmap phase is Linux computer use, only after a separate scoped request and isolation
review; this implementation does not silently proceed to it.

See [model policy](LLM_PROVIDERS.md), [official evidence](OPENCODE_GO_RESEARCH.md),
[agent architecture](AGENT_ARCHITECTURE.md), [migrations](MIGRATIONS.md),
[testing](TESTING.md), and [roadmap](ROADMAP.md) for canonical contracts.
