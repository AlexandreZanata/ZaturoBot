# Foundation validation and optional resource benchmark

The primary test suite is Rust: run `cargo xtask check --locked` from the repository
root. It runs domain, persistence, protocol, policy, worker recovery and local
hub-to-worker integration tests, plus documentation and dependency contracts. Tests
use temporary databases and local synthetic data; no paid model, account credentials,
or production repositories are required.

## Reproduce Linux memory and load measurements

Build the actual release binaries, then run the standard-library-only benchmark:

```bash
cargo build --release --locked -p zaturo-hub -p zaturo-worker
python3 tests/benchmark.py --tasks 30 --concurrency 4 --output /tmp/zaturo-benchmark.json
```

The script runs two local child processes with separate temporary SQLite databases,
random credentials supplied only through environment variables, private configuration
files, and an ephemeral loopback port. It submits diagnostic tasks through the
authenticated HTTP API and verifies their actual diagnostic results. Submission
concurrency is four by default; worker execution remains one task at a time. All
processes and temporary files are cleaned up, including on failures. Child logs are
suppressed so the JSON report contains no authentication headers or credentials.

The hub receives an `RLIMIT_AS` of 300 MiB and `MALLOC_ARENA_MAX=2`. This limits virtual
address space, which differs from resident memory and from a cgroup memory cap.
The worker is outside that limit. The report samples the hub's `/proc/PID/status`
`VmRSS`, `VmHWM` and `VmSize` every 50 milliseconds; `VmHWM` supplies the kernel's
resident high-water mark as observed during the run. The report includes an idle
sample after worker registration and one second of warm-up, observed peak memory,
workload timing, database file sizes, environment details and executable hashes.
The sampler stores aggregate statistics with bounded memory.

By default, the script restarts the hub after completion, verifies durable completed
task metadata and one task's event history, and waits for the same worker to
reconnect. This check exercises persistence after process termination; it does not
replace the Rust tests for interruption during active execution. Use
`--no-restart-check` to omit it. See `python3 tests/benchmark.py --help` for workload,
sampling, binary directory and address-space limit options. Task counts are capped
at 200 to stay within the diagnostic task deadline and keep the test bounded.

The script never builds automatically or accesses the public network. A restricted
execution environment may require permission to bind a loopback listener. A failure
to start under a specific memory limit is a real benchmark failure, and must be
reported with that environment rather than replaced by an assumed result. Results
represent the recorded synthetic workload, not a promise about production capacity.
