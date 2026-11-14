# Device file engineering report

Version 0.3.0 implements the owner-authorized Ubuntu/Debian device relay described in
[file transfers](FILE_TRANSFERS.md). Mouse/desktop control, arbitrary task execution,
browser research and social-account integration remain future scope.

## Automated verification

`cargo xtask check --locked` passed with 107 tests, strict Clippy, formatting,
architecture/documentation/registry contracts, secret/history checks and rustdoc.
Network fixtures run on literal loopback with generated age identities and synthetic
tokens; no production API or paid model is used in tests. The restricted execution
sandbox denies opening local sockets, so the full networking gate ran with the required
network permission rather than weakening or skipping those tests.

Behavioral coverage includes bidirectional actual HTTP/WS delivery, wrong credentials,
fixed device hello identity, owner/device separation, interrupted progress, exact replay,
orphan bytes, quota/idempotency/retention, hub 1-to-4 migration preserving a worker,
wrong age identity, tampering, empty plaintext, traversal, symlinks, FIFO rejection,
process cache locks, no overwrite and hidden filenames. A full pending page of invalid
age files does not starve a subsequent valid arrival. Transient HTTP retries are
bounded to four; an authorization denial is attempted once.

`cargo build --release -p zaturo-hub -p zaturo-worker --locked` built locally. The private
installer was checked for protected output, source/version checks and idempotent repeated
installation into an isolated simulated home. Credentials and the real work-PC handoff
stay under ignored `.secrets/`; only code, templates and operational documentation are
published. `cargo xtask secrets --locked` verifies visible work, staging and history.

## Live Oracle and home-PC verification

The hub runs as an unprivileged system service, bound to loopback, with its existing
firewall unchanged. State/spool are 0700, registry is root:hub 0640, and registry entries
contain only bearer digests and public age recipients. Hub schema upgraded from 1 to 4;
all three existing task records were retained. The prior schema-1 snapshot and 0.2 hub
binary were preserved. Home worker journal schema remains 3 and task protocol remains 2.
Home user services are enabled with lingering and a separate inbox loop.

New independent SSH keys were issued for home and work. The `zaturo-link` account passed
an actual hub-forward test. Shell, another remote destination port and remote forwarding
were actually refused. The existing admin account remained permitted, verified through
effective configuration and subsequent admin connections. The home service now uses its
restricted key rather than the admin key. Strict host verification remains enabled.

Actual production-path tests use two profiles on the home PC, independent device tokens,
SQLite journals/caches and separate restricted SSH tunnels through the VPS. Each run
completes a work-profile diagnostic and verifies 64 MiB home-to-work plus 8 MiB
work-to-home, including automatic inbox receipt and exact plaintext hashes.

The initial 1 MiB-chunk upload took 20.422 seconds for 64 MiB. With 4 MiB chunks the
same-size test took 11.528 seconds (5.552 MiB/s, 46.571 Mbit/s), and verified end-to-end
receipt took 26.331 seconds including the second hop and inbox polling. These are
measured values on this connection, not a speed promise for the physical work network.
The final installed build repeated the test in 11.962 seconds for upload (5.350 MiB/s,
44.880 Mbit/s), with verified end-to-end receipt in 25.772 seconds. Its work diagnostic
`2f002a1c-8446-42fd-8137-a680d5590923` completed; both transfer directions were delivered.
Home worker peak cgroup memory was 25,300,992 bytes and its tunnel peak was 8,265,728
bytes, with both active and zero systemd restarts. Lingering remained enabled. Hub peak
cgroup memory was 109,219,840 bytes, including charged file cache, below the 300 MiB
service cap. Its installed SHA-256 was
`b661c82ee190219842cf33c48e35de6e9a3454dbc8ca90bcb6c29a0d8b64279a`.
The live hub reported six completed tasks, seven delivered synthetic transfers, schema 4
and SQLite integrity `ok` after the final smoke.

A separate 32 MiB upload was deliberately killed after 8,388,608 ciphertext bytes were
checkpointed. Restarting the hub preserved that exact offset. Resuming the same request
completed delivery with the original plaintext SHA-256; resume plus receipt took
16.456 seconds. File tests made no additional AI calls: the home model ledger remained
at its original two settled calls and 931 charged tokens.

The unrelated local PDF project, certificates and original SSH files were not modified
by this module. The previously encrypted retirement backup remains private.

## Physical destination handoff

The work-PC profile is enrollment-ready, not installed on the unavailable physical PC.
The protected prompt includes a new scoped SSH key, device bearer, age identity, pinned
host key, exact source commit and Linux installation/acceptance steps. It excludes the
VPS admin key, owner API token and AI provider key. Its simulated worker/tunnel are stopped
before handoff. A small encrypted challenge is staged for the actual work PC with the
normal 24-hour relay lifetime. Its Codex must validate receipt, send a nonsecret return
file, complete its diagnostic and verify user-service persistence on the actual network.
