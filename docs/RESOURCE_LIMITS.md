# Resource limits and measurements

The Oracle hub has 1 GB RAM and constrained CPU/storage. Design targets are normal hub
RSS well below 150 MB and peak below 300 MB. Local measurements below do not
guarantee production capacity.
Do not run inference, graphical sessions, browsers, external project builds, video
rendering, or heavyweight scanning there. Build release artifacts elsewhere.

The notebook has 22 GB RAM; its GPU is unknown. Use bounded concurrency and owner-selected
CPU/RAM limits. The foundation executes one short deterministic diagnostic at a time.
Future tools need per-process time/memory/CPU budgets and owned process-group cleanup.
Phase 2 request/task/day/month token and model-call limits are enforced through the
worker ledger; provider selection and accounting semantics are in
[LLM providers](LLM_PROVIDERS.md).

## Enforcement and capacity

Configuration validation defines allowed bounds. Queue admission, paginated history/list
queries, deadlines, retries, reconnect backoff, and typed output budgets constrain work.
The production listener admits at most 32 TCP sockets before parsing HTTP headers or
authentication and closes connections after 30 seconds without actual read/write
activity. A permit remains held through WebSocket upgrade until the socket is dropped.
The kernel listen backlog is separate from admitted application sockets.

HTTP handling admits at most 32 in-flight requests with a 10-second request deadline.
WebSocket permits use `max_connections` (eight in the example), while only one configured
worker identity may register an active session. WebSocket frames/messages and incoming
HTTP bodies are capped at 16 KiB. Paginated HTTP responses can exceed 16 KiB; their
record counts and each record's input/result sizes remain bounded. Hello and WebSocket
writes have 5-second deadlines. Service settings and source constants are authoritative;
do not add competing definitions in deployment profiles.

Task creation/control/completion notify the connected worker promptly. A 5-second
fallback checks durable state and lease recovery, so a lost in-memory notification cannot
lose queued work. Skipped missed ticks avoid accumulating timer work. This is bounded
queue maintenance, not a recurring-job scheduler.

The systemd examples contain service memory/CPU caps and log-rate limits. Caps contain
resource use and may terminate a process; they do not prove a workload fits. A private
TLS proxy can add connection/header/idle limits as defense in depth. Protect host file
descriptors, journald storage, and database disk capacity separately.

The model path additionally bounds context/response bytes, provider timeout, attempts,
concurrency, and planner steps. The hub never performs inference or stores its API key.
A paid timeout is retained in the ledger, not reclaimed as free capacity.

## Reproducible measurement

Build release binaries and run the standard-library-only local benchmark:

```bash
cargo build --release --locked -p zaturo-hub -p zaturo-worker
python3 tests/benchmark.py --tasks 100 --concurrency 4 --output /tmp/zaturo-benchmark.json
```

The [benchmark procedure](../tests/README.md) documents temporary databases, synthetic
credentials, loopback transport, task acceptance, memory sampling, restart checks, and
cleanup. The [recorded local result](benchmark-local.json) completed 100 diagnostics in
9.103539 load seconds with four concurrent submissions and one executing worker task.
Idle hub RSS was 8.3046875 MiB; observed peak RSS/kernel high-water mark was 9.109375 MiB.
The hub ran with a 300 MiB `RLIMIT_AS` virtual-address-space limit and
`MALLOC_ARENA_MAX=2`. This was not a cgroup/RSS memory cap; the worker was outside it.

The environment was x86_64 Linux, kernel `7.2.15-76070215-generic`, Rust 1.99.0,
16 logical CPUs, and 33,337,683,968 bytes of host RAM. The artifact records executable
hashes, 50 ms samples, database/WAL sizes, and verified hub restart/worker reconnect.
This was a development-machine synthetic workload, not the Oracle VPS. It establishes
observations for that run, not target-machine capacity, sustained memory behavior,
long-running retention, or future-tool performance. A measured deployment workload and
separate cgroup/systemd constrained run remain necessary before a production claim.
