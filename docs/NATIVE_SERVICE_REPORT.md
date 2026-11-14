# Native clients and typed services: 0.4 engineering report

Version 0.4 implements the owner's specific native-client, persistent activity,
YoutubeBot integration, private-data handoff, and Android USB foundation requests.
It extends the durable hub/worker/file system; it does not claim complete later roadmap
phases, unrestricted automation, or real-phone/media acceptance that did not occur.

## Implemented behavior

The Linux client is native Rust/Slint; the Android companion is native Kotlin, not a
webview wrapper. Shared branding uses the supplied icon and black/gold design tokens.
Both clients use distinct scoped application credentials and the existing private hub.
Ordinary chat/activity persists in hub SQLite with current-day defaults, bounded pages,
day/type/search filters, idempotent submissions, and deduplicated terminal notices.
Historical days remain stored. Arbitrary text is recorded as a request; only explicit
supported actions create executable typed tasks.

The desktop uses the existing pinned tunnel or a private HTTPS origin. Android imports
a separate protected connection into Keystore-encrypted storage and owns a host-pinned
restricted SSH connection on port 22. It does not receive an admin, worker, provider,
or another computer's decryption credential. Client lifecycle bounds polling/connection
ownership; ordinary durable jobs survive a client closing.

YoutubeBot remains the video implementation in its own repository. The original local
checkout and its extension, recorder, generation, prompts, upload behavior, and private
files were preserved. The authorized isolated checkout gained fixed JSON service verbs,
a durable operation/actor ledger, independent account/channel locks, status reconciliation,
account setup UI, and bounded completion observation. The ZaturoBot adapter invokes that
fixed interface rather than reimplementing the pipeline or accepting a shell command.
Heavy jobs use separate fixed user units; a worker acceptance does not mean a video
completed. Uncertain external submissions reconcile by operation status, never blind
resubmission. Google/ChatGPT sign-in and exact channel verification remain interactive.

Android USB tools bind an explicitly registered serial and enforce local grants. Device
status and finite private capture/change-watch are typed operations. Sensitive PNG bytes
are encrypted in worker RAM to the bound desktop endpoint. The hub holds bounded
ciphertext briefly in RAM for one retrieval, separate from SQL, artifacts, logs, normal
feed, and durable file spool. The desktop decrypts in memory, bounds image allocation,
and clears its private panel on expiry/focus loss/close. Owned plaintext buffers zeroize;
this does not promise erasure of every OS/compositor rendering copy.

The private viewer is Linux-desktop-only. Android mobile does not decrypt/render these
age images. The phone must be authorized, connected, already unlocked, and on the
permitted screen; for SMS viewing its SMS app must already be open. A finite byte-level
screen-change watch can be triggered by a clock/animation and is not an SMS semantic
arrival detector. There is no OCR, OTP extraction/use, notification harvester, automatic
account sign-in, lock-screen bypass, or protected-window bypass.

## YoutubeBot revision and destination handoff

The reviewed YoutubeBot main revision is
[`89dc1e604b87084cfdfc7191058880e7ddf080a5`](https://github.com/AlexandreZanata/YoutubeBot/commit/89dc1e604b87084cfdfc7191058880e7ddf080a5).
Its [main CI run](https://github.com/AlexandreZanata/YoutubeBot/actions/runs/43546881432)
is green, with 153 Python tests and 30 Node tests. These are that repository's checks,
separate from the ZaturoBot gate below. No account sign-in or live upload is inferred
from synthetic CI. The pinned additive installation is documented in
[work services](../deploy/services/WORK_SERVICES.md).

The private migration snapshot inventory contains **12,455 files totaling
8,899,234,841 bytes**. All **eight encrypted archive parts and three metadata deliveries**
actually reached the already-enrolled `work-pc` through the file relay. Sender originals
were retained. Checksummed, bounded no-clobber restoration stages old source overlays
for review instead of overwriting the improved source. Existing destination browser
profiles/conflicts are preserved; sessions may need interactive reauthentication.

The existing work-PC 0.3 enrollment was already confirmed operational. Its SSH/age keys,
worker credential, inbox, and execution journal must stay in place. Destination restoration
and the pinned 0.4 delta run through its other Codex using the
[restore helper](../deploy/services/README.md) and additive updater. This report does not
claim the work PC has applied that update or restored the snapshot. No re-enrollment,
credential replacement, original-data deletion, or automatic execution of received files
is required or authorized by a successful delivery.

## Validation actually performed

The latest `cargo xtask check --locked` completed successfully. Its recorded results
contain **149 passing Rust tests** and **21 passing Python helper tests**, with zero
failures. Formatting, strict Clippy, architecture/documentation/registry contracts,
secret/publication guard, and rustdoc passed as part of the same gate. The Rust total
is the sum of the actual `test result` records; the Python helper count is separate.

Coverage includes scoped client authentication and day-filtered/idempotent activity,
terminal notice deduplication, typed service input/output, durable service uncertainty,
worker isolation, private-view grants/revocation/expiry and one-time encrypted retrieval,
USB selectors/leases, file bounds/tampering, migration preservation, and the existing
model/task contracts. Python helper fixtures cover verified restore/no-clobber behavior
and additive update backup/quiesce/publish sequencing without real accounts or phones.
No production service or paid model is part of the ordinary automated gate.

The Rust/Slint desktop release was built and installed. Its actual scoped feed, workers,
and services requests returned HTTP 200 with no sensitive feed rows. The installed native
window rendered in a separate Xvfb display, remained running, and exited successfully;
this did not operate the owner's normal desktop. The Android debug APK was built, four
Android JVM tests passed, and Android Lint reported zero errors. Its restricted SSH JVM
smoke actually reached the private hub and tested wrong-host-pin refusal. A separate
live JVM check used the production SSH transport and HTTP client: scoped feed returned
200 with no sensitive rows, owner task creation was denied with 401, and an incorrect
host pin failed before authentication. These are build/transport results, not a physical Android test;
real-device Keystore, lifecycle/reconnection, USB behavior, and layout acceptance remain.
The source/build/enrollment procedures are in the
[desktop README](../apps/zaturo-desktop/README.md) and
[Android README](../mobile/android/README.md).

The branded [system design PDF](../output/pdf/zaturobot-system-design.pdf) was finalized
as six pages and rendered for visual review. Its [design notes](design/README.md) describe
editable source, embedded fallback fonts, conceptual layout, and evidence boundaries.
It contains no deployment secrets, host address, or account identity.

## Verified hub/home deployment and cross-PC smoke

The Oracle hub is now running version 0.4 with hub schema 5 and SQLite integrity `ok`.
A consistent pre-upgrade schema-4 snapshot was retained. At least eight existing tasks,
two workers, 20 transfers, and 60 transfer events survived the upgrade. The control listener
remained private `127.0.0.1:7878`; no new public port was opened. A post-restart cgroup
`MemoryCurrent` observation was 2,650,112 bytes against its 300 MiB service cap. This is
not an RSS or load measurement and does not establish production capacity.

The home worker is version 0.4 with journal schema 6 and a retained consistent schema-3
snapshot. Eight preexisting private profile files were byte-identical after installation;
its model ledger remained at two calls. The new scoped desktop app environment was
separate. Task protocol remains 2. Both registered workers heartbeat, and the existing
work-PC 0.3 `diagnostic.v1` remains compatible without replacing its credentials/journal.

An actual scoped-desktop-app `POST /v1/chat` created a diagnostic targeted at that existing
work PC. It reached `COMPLETED`, and a day-filtered persistent task-completion notice was
verified. The same app credential was denied with HTTP 401 at the owner `/v1/tasks`
creation endpoint. This validates a real cross-PC execution/feed path and role boundary,
not just a systemd active state. It made no model call.

The Slint desktop release, scoped app environment, menu entry, and supplied icon were
installed on the home PC. The Android debug APK was copied to protected private output.
The already-operational work PC still needs its destination Codex's additive 0.4 update
and no-clobber private restore; it has not been re-enrolled by this operation.

No physical-phone acceptance, real Google consent, or new video generation/upload is
claimed by these checks. Earlier foundation/0.3 measurements remain historical workloads
rather than 0.4 capacity proof. Matching older binaries/configuration and verified SQLite
snapshots are required for rollback; a binary replacement alone is insufficient.
See [live operations](LIVE_DEPLOYMENT.md) and [migrations](MIGRATIONS.md).

## Security and remaining work

Application, owner-admin, worker/device, SSH transport, file encryption, and model
credentials retain distinct boundaries. Real credentials, host addresses, private
archives, browser profiles, and enrollment files remain private and untracked. Publication
requires `cargo xtask secrets` plus review of visible/staged/history changes. Service
capabilities are explicit; scheduling a YouTube batch requires the configured grant and
typed owner approval. Chat text, restored files, model output, and phone pixels cannot
grant permissions. The VPS executes coordination and ciphertext relay only.

The remaining acceptance tasks are destination Codex additive installation and private
restore, interactive account/channel setup, an
explicitly authorized real media run, Android APK import/foreground lifecycle on a real
phone, and owner-selected USB/private-view checks. Full desktop input, continuous remote
streaming, general vault, arbitrary account actions, and recurring scheduling remain
planned. Update the [roadmap](ROADMAP.md) only for concrete verified behavior.
