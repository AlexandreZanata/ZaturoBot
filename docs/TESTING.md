# Testing and validation

`cargo xtask check` is the repository/CI gate. It runs formatting, clippy with denied
warnings, workspace tests, rustdoc, dependency checks, documentation/registry checks,
and the bounded Git-visible publication-secret check.
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

Phase 2 adds fake-model and synthetic HTTP fixtures for malformed actions, forbidden
tools/routes, exclusive model selection, privacy denial, token accounting, bounded
retries, cancellation, and uncertain paid-request recovery. SQLite tests exercise
reservation races, UTC day/month limits, budget debt, and worker schema upgrades.
An opt-in paid smoke test needs explicit owner authorization and is separate from CI.
Source fixtures contain synthetic data only.

`cargo xtask secrets` checks nonignored Git-visible working files, staged blobs, and
reachable historical blobs and commit/tag messages. It rejects known private/runtime paths, private-key blocks,
common provider-token prefixes, and literal credential assignments without echoing
values. Ignored `.env` and `.secrets/` are never opened. Mutation tests prove that a
secret left staged, in a commit message, or in older history still fails after cleaning
the working file. This
heuristic guard is not proof that every possible encoded secret has been detected.

CI checks out reachable history (`fetch-depth: 0`) so its publication scan does not
silently inspect only a shallow tip. Object/list/history limits fail explicitly rather
than loading arbitrary artifacts without bounds.

A missing future feature has a planned acceptance contract, not a passing fake test.
Vault encryption, recurring scheduling, desktop control, and personal integrations
remain absent until their own behavioral evidence exists.

Memory/load evidence is separate from correctness tests; see
[resource limits](RESOURCE_LIMITS.md). Record commands actually run and environmental
constraints in the engineering report. CI configuration existing locally is not proof
that a remote GitHub Actions run has passed.
