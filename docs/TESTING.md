# Testing and validation

`cargo xtask check` is the repository/CI gate. It runs formatting, clippy with denied
warnings, workspace tests, rustdoc, dependency checks, and documentation/registry checks.
Run focused package tests while editing, then the gate once the change is complete.

Tests are executable contracts. Use injected UTC times, temporary SQLite files, synthetic
identities, fake credentials, and loopback servers. They do not need real model keys,
cloud services, production repositories, Gmail, video uploads, or general network access.
Package-level tests live next to their owners; cross-component/transport tests belong in
the apps. The loopback integration test runs the actual hub and worker libraries in one
test process; separate CLI processes are exercised by the local demonstration. Repository-check mutation tests must prove invalid structures are detected.

## Required evidence for the implemented slice

- Domain: legal/illegal transitions, terminal behavior, typed diagnostic acceptance.
- Database/services: atomic task/event writes, migrations, queue/claim bounds,
  capability matching, lease expiry, deadlines, retry caps, idempotency, pagination,
  cancellation/pause/resume, and stale completion rejection.
- Worker journal: persist intent/result across reopen; reuse completed diagnostic evidence;
  do not treat transport disconnection as proof an operation did not happen.
- Protocol/security: round trips, unsupported/malformed input, authentication role failure,
  invalid configuration, secret redaction, and denied unregistered tools.
- Transport: local hub/worker lifecycle with persisted history, disconnect/reconnect, and
  hub/worker restart during or around assignment.

A missing future feature has a planned acceptance contract, not a passing fake test.
There is no LLM adapter, vault encryption, recurring scheduler, desktop, or integration
whose real behavior can be claimed by this suite. Fake-model tests become mandatory in
Phase 2. Source fixtures must contain synthetic data only.

Memory/load evidence is separate from correctness tests; see
[resource limits](RESOURCE_LIMITS.md). Record commands actually run and environmental
constraints in the engineering report. CI configuration existing locally is not proof
that a remote GitHub Actions run has passed.
