# Live hub, workers, and native-client rollout

This runbook owns current operational paths and rollout status. The implementation and
validation record for 0.4 is the [native/service report](NATIVE_SERVICE_REPORT.md).
[Phase 2](PHASE2_REPORT.md) and [file-transfer](FILE_TRANSFER_REPORT.md) reports retain
historical observations; their earlier handoff/schema statements are not current rollout
claims. Operational secrets, addresses, enrollment bundles, and backups remain private.

## Current rollout record

The Oracle hub and home-PC worker have operated as independent systemd services since
the earlier authorized activation. The existing work PC is already enrolled and working
on 0.3; its identity, SSH/age keys, inbox, and journal remain valid. It must receive an
additive pinned 0.4 update through its destination Codex, not another enrollment.

The Oracle hub and home worker are now verified on 0.4. The hub is active with schema 5
and SQLite integrity `ok`, after a consistent pre-upgrade schema-4 snapshot. The update
retained at least eight existing tasks, two workers, 20 transfers, and 60 transfer events.
The home journal is schema 6 with its consistent schema-3 snapshot retained. Eight
existing private profile files remained byte-identical, and the model ledger stayed at
two calls. Both workers heartbeat; the existing work-PC 0.3 diagnostic remains compatible.

A real scoped-desktop-app request created a diagnostic on the existing work PC. The task
completed and its persistent day-filtered completion notice was retrieved. The same app
credential received HTTP 401 at the owner task-creation endpoint, confirming role
separation. This smoke used no model call. The home Slint binary, scoped app environment,
and menu/icon were installed; the Android debug APK was copied to private output.
The installed desktop's scoped feed/workers/services requests returned HTTP 200 with no
sensitive feed rows; its native window rendered on a separate Xvfb display and exited
successfully. Physical Android acceptance and the work-PC 0.4 delta remain outstanding.

All eight encrypted YoutubeBot archive parts and three metadata deliveries reached the
already-enrolled work device. The snapshot inventory contains 12,455 files totaling
8,899,234,841 bytes. Delivery does not imply that its private restorer ran or its 0.4
service update was applied. Use the [work-service runbook](../deploy/services/WORK_SERVICES.md)
and [no-clobber restore contract](../deploy/services/README.md).

## Private network path

The VPS hub binds `127.0.0.1:7878`. Each enrolled Linux PC initiates an outbound SSH
connection on port 22 and forwards local `127.0.0.1:17878` to that remote loopback
address. HTTP/WebSocket traffic remains inside the encrypted cross-network path.
No home-router forwarding, public hub listener, or new web firewall allowance is needed.
Direct remote plaintext HTTP remains rejected by application origin validation.

Per-device restricted keys use the separate `zaturo-link` account, with local TCP
forwarding permitted only to the hub. Admin access stays separate. Strict host checking
uses a pinned known-host file; changed keys require independent verification. The VPS
firewall keeps deny-incoming policy and its existing SSH allowance. Credentials, actual
host addresses, and private keys never enter public templates or diagnostic output.

Android has its own owner-issued restricted SSH identity and host pin, imported through
Keystore-encrypted enrollment. Its forward is local `127.0.0.1:17879` to the same private
hub. The live JVM transport smoke tested this restricted path and wrong-pin refusal;
physical Android/Keystore/UI/lifecycle acceptance has not occurred.

## Installed service boundaries

The hub uses the unprivileged `zaturo-hub` account. A post-restart cgroup
`MemoryCurrent` observation was 2,650,112 bytes against the 300 MiB service cap; this is
not RSS, a load benchmark, or a capacity guarantee. Its only control listener remained
private loopback. Installed paths:

- Binary: `/usr/local/bin/zaturo-hub`.
- Enabled unit: `/etc/systemd/system/zaturo-hub.service`.
- Configuration: `/etc/zaturo/hub.toml`, `root:zaturo-hub`, mode `0640`, under `0750`.
- Service environment: `/etc/zaturo/hub.env`, root-owned mode `0600`.
- State: `/var/lib/zaturo-hub/hub.db`, under a hub-owned `0700` directory.
- Device registry: `/etc/zaturo/devices.toml`, `root:zaturo-hub`, mode `0640`.
- Ciphertext transfer spool: `/var/lib/zaturo-hub/transfers`, mode `0700`.

Native-client activation additionally requires a private `apps_file` registry containing
scoped application bearer digests and any explicit viewer/device binding. It contains no
provider key or private age identity. Application access is separate from owner-admin and
worker/device authentication. Sensitive screen ciphertext uses bounded expiring RAM,
not the ordinary durable transfer spool.

The home PC uses owner-account user services because local administrative sudo is
unavailable:

- `zaturo-hub-tunnel.service`: [template](../deploy/notebook/zaturo-hub-tunnel.user.service).
- `zaturo-worker.service`: [template](../deploy/notebook/zaturo-worker.user.service).
- Binary: `~/.local/lib/zaturobot/zaturo-worker`.
- Protected config/environment: `~/.config/zaturobot/`, directory `0700`, files `0600`.
- State/workspace/artifact roots: `~/.local/share/zaturobot/`.
- Convenience wrapper: `~/.local/bin/zaturo`.

Lingering is enabled so user services outlive an interactive login. Worker/tunnel
reconnection and systemd restart are independent of this chat. No isolated desktop user
or full computer-control session is implied by these headless services.

## Credentials, models, and heavy jobs

`owner.env`, `worker.env`, `tunnel.env`, and native `app.env` separate operational roles.
The hub holds service/application digests and coordination data, not Google, ChatGPT,
YouTube, or OpenCode Go account credentials. The provider key remains on the home worker.
No new provider key is needed on the work PC for file, YoutubeBot, or USB foundations.

Only `deepseek-v4.1-flash` is allowed for the bounded assisted diagnostic. Its protected
profile explicitly permits public-coding/Global processing and denies private personal
content and training-enabled routes. Screens, chat/account data, and migrated documents
are not automatically model inputs. Read [provider policy](LLM_PROVIDERS.md) before
changing this. Model budgets/reservations remain durable through worker updates; an
uncertain paid call is not free and must not be replayed automatically.

YoutubeBot browser/video jobs run under separate fixed transient user units, not the
small diagnostic worker's memory cgroup. The configured bounds are 12 GiB MemoryHigh,
16 GiB MemoryMax, CPUQuota 200%, TasksMax 256, a 12-hour maximum, and no automatic restart.
A submission acknowledgment is not video completion. Status reconciliation and verified
final observations determine completion notices. The VPS runs no media, model, browser,
ADB, or desktop workload. Real Google consent/channel verification and media acceptance
remain interactive/explicit, not automated fixture tests.

## Update and recovery

Build release binaries on a compatible PC, never on the constrained VPS. Stop/quiesce
the affected process before a verified SQLite backup and replacement, retaining its
matching binary/configuration. Version 0.4 upgrades hub schema 4 to 5 for chat/activity
and worker schema 3 to 6 for the independent service journal. Task protocol remains 2.
Existing task/file records, model reservations, and execution fences are preserved.
A rollback needs the matching older schema snapshot; replacing a binary alone is unsafe.
See [migrations](MIGRATIONS.md).

The work-PC helper builds/prepares before stopping its existing worker, takes a consistent
backup, publishes the reviewed update, and preserves credentials/journal/inbox. If an
update fails after quiescing, inspect its private recovery marker and live schema;
do not delete operation evidence or automatically restore an old binary over a new DB.
The required YoutubeBot revision and passing CI are pinned in the
[work-service runbook](../deploy/services/WORK_SERVICES.md).

After an authorized update, verify the installed binary version/hash, actual schema and
integrity, tunnel/worker connectivity, a completed authenticated diagnostic, file inbox
operation, and the explicitly enabled service capabilities. Native application credentials
must be tested against scoped routes without granting owner-admin lifecycle APIs.

## Routine operation and acceptance limits

```bash
systemctl --user status zaturo-hub-tunnel.service zaturo-worker.service
journalctl --user -u zaturo-worker.service -n 30 --no-pager
loginctl show-user "$USER" -p Linger
~/.local/bin/zaturo files peers
~/.local/bin/zaturo task diagnostic
```

On the VPS, use `sudo systemctl status zaturo-hub` and bounded structured journald output.
Never print environment files or authenticated headers. For source owner-CLI operations,
use the ignored client profile targeting the existing local SSH forward; do not start a
second hub with that client profile.

The desktop/Android builds and synthetic/private-transport smokes do not establish
physical-phone operation, Android Keystore persistence, account consent, or a real video
upload. The private phone viewer exists only on Linux. It needs an explicit serial-bound
grant and an unlocked permitted screen; an SMS screen must already be open. A finite
image change does not mean an SMS or OTP was detected. No OCR, code extraction,
automatic sign-in, protected-screen bypass, or continuous remote desktop is implemented.

The earlier PDF retirement and encrypted backup remain in the historical
[Oracle reuse report](../deploy/oracle/REUSE_VPS.md). Local PDF files/certificates and
original local YoutubeBot data remain preserved. General vault, full desktop automation,
recurring scheduling, and arbitrary account/project actions remain outside this rollout.
