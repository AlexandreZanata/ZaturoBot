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


0.4 fixtures add local-midnight/Unicode/literal daily history, atomic chat and stable notices,
legacy schema/journal preservation, service intent/reconciliation and actual job evidence,
USB authorization/serial/timeout/process-group/lease tests, and full fake-ADB -> worker ->
hub RAM agecipher -> owner-memory decrypt/once/expiry/cancel tests. Native desktop tests
exercise bounded HTTP/private paths; an isolated-Xvfb fixture visibly renders an ephemeral
synthetic PNG and proves earlier server TTL clearing/DELETE without plaintext files.
Android JVM tests check origin/enrollment and canonical SSH-wire fingerprints; an explicit
opt-in smoke uses a new restricted key against hub healthz and rejects a wrong host pin.
APK compilation/Lint are not physical Android acceptance. Google/realUSB/paidgeneration
are never CI requirements. See [native clients](NATIVE_CLIENTS.md) and adapter tests.


## Native timeline and presentation acceptance

The [shared native contract](NATIVE_CLIENTS.md) governs Android and Linux. Exercise
behavioral transitions rather than tests that merely duplicate a label/value table:
newest-first pages become chronological rows; stable IDs deduplicate; sensitive rows
stay excluded; raw page counts advance offsets even when visible counts differ; and
older-page merges remain bounded to 200 ordinary rows.

Cover Hub open, reopening after older-history reading, incoming updates near and away
from the bottom, and accepted own sends from an old day/type/search. Acceptance of a
send must return to today/all kinds/empty search, fetch the fresh page, and retain an
explicit bottom request even when the returned rows are unchanged. A passive refresh
must not consume that intent prematurely or steal composer focus. Prepending older rows
must retain the visible reading anchor. Drafts remain in RAM across tab changes, and
sensitive data must never enter the ordinary timeline or draft fixtures.

Pure timeline tests do not prove a viewport reached its final bottom. Separate native
UI checks must wait for actual layout/measurement, inspect the last visible ordinary
row after open/send, and verify anchor/focus behavior during older-page and unchanged
refresh cases. Use isolated Xvfb and a fresh private Android emulator with synthetic
content. Check outlined/no-shadow controls, 48dp mobile action targets, official icon
tints, right-gold/left-dark bubbles, keyboard insets, and desktop sidebar sizing at the
available window dimensions. Those tests do not operate the owner's normal desktop or
use a real SMS screen.

An explicitly authorized physical Android run can validate the particular APK, device,
Keystore connection and actual layout. Record it only after observed success; an emulator,
APK build, unit-test pass or older physical run does not establish acceptance of a newer
UI revision. Existing API/authentication, model and ephemeral private-view tests remain
required because presentation changes must preserve those boundaries.


## Assistant and shared-area extension

The 0.5 gate adds aggregate coding/personal budget admission and uncertainty across
restart, preserved schema upgrades, bounded conversation context, typed zero-model
commands, shared-area revision/idempotency/authentication fences and compact stdio MCP.
Fake YoutubeBot/ADB fixtures distinguish generation from scheduling and test precise
login/USB states. Native app-viewer tests bind public keys to authenticated identities,
enforce one-time encrypted content and exclude private frames from history.
Android JVM fixtures cover standard-age interoperability, wrong recipients, tampering,
and bounded decoding; opt-in age CLI interoperability uses synthetic files only.
A live Console check is separate, owner-authorized and charged through the normal worker
ledger. No fixture substitutes for physical work-PC USB/browser acceptance.


Private-session integration runs the production worker reconnect loop: consuming/revoking
a frame can race the completion acknowledgement, and the protocol deliberately closes
an uncertain connection for reconciliation. A single `connect_once` fixture cannot assume
continuity across that race. Keep recipient/one-time/no-disk/unchanged-watch/cancellation
assertions intact and verify that the subsequent session completes within its bound.
The work-services updater also tests all nine typed capabilities together, retained grants,
private scoped MCP installation and refusal to rebind an existing USB serial.


Installed-executor fixtures use local HTTP, synthetic usage, fake budgets and metadata.
They verify one forward despite retry, billing rejection before tool release, strict
arguments, typed intent authority/idempotency, navigation/lock evidence and portal source
bounds. They never call production Go, YoutubeBot, ADB, accounts or capture a desktop.
Physical acceptance is separately owner-authorized and recorded with exact identities;
see [the acceptance contract](OPENCODE_EXECUTION.md). Android source/build validation must
run in the existing SDK environment without replacing the enrolled signing identity.
