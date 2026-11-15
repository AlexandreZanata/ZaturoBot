# Add services to an already-enrolled work PC

This is an additive **0.3/0.4/0.5 → 0.6** update, not another enrollment. Run it as the existing
Linux owner, never root. Keep the current `work-pc` worker ID, private SSH key and pinned
host, age identity, worker token, transfer inbox and execution journal. The updater never
replaces those credentials or removes original YoutubeBot files. The private archive
restore is a separate, no-clobber operation described in [README.md](README.md).

## Before the update

Use clean source checkouts at the full revisions in the owner-issued private enrollment.
The reviewed YoutubeBot revision is `bec3a3ffc222c9cc9074faa6560fbab00d41aaf9`.
Its local verification passed 158 Python tests and 30 Node tests. The
[main CI run](https://github.com/AlexandreZanata/YoutubeBot/actions/runs/43603001353)
passed for that revision. The ZaturoBot revision is supplied after publishing; do not substitute a moving branch
name or hardcode the installer's own revision. Ignored runtime data/build outputs may
exist, but tracked edits and untracked source files cause rejection. Preserve an older
working checkout and its local data; use a separate pinned clone for the updated service.

Receive the work-services enrollment through the **existing encrypted file relay**.
It contains only `schema: 1`, `app_token`, `expected_zaturo_revision` and
`expected_youtube_revision`. Keep it mode 0600 in a private directory. The token is a
new scoped application credential bound to `work-pc`, not an owner/admin, worker, SSH,
age or AI-provider secret. Do not paste credentials in a command, Git, issue or shell
history. No provider key is required on the work PC. Any existing `models_config` and
worker environment are preserved.

Use Python 3.11+ (YoutubeBot's tested desktop environment is Python 3.12), its `venv`
module, the Rust toolchain pinned by `rust-toolchain.toml`, Cargo's native Linux build
prerequisites, and a working user systemd manager. Source builds may need network access
for locked Cargo dependencies. Installing system packages remains an explicit OS setup
step; this helper does not run sudo or change the user's login/linger policy.

For full media execution also install the existing YoutubeBot requirements: Chrome,
ffmpeg, Xvfb and audio tools/runtime libraries appropriate to this Linux desktop. Follow
YoutubeBot's current README and pipeline setup for exact environment checks. Desktop
native rendering needs the normal Linux X11/Wayland, XKB/font and graphics runtime.
USB support additionally needs `/usr/bin/adb` and the owner's USB authorization/udev
permissions. No public ADB port or new public hub port is introduced.

## Review then apply

Use absolute paths; the placeholders below are paths, never credentials:

```bash
python3 /absolute/ZaturoBot/deploy/services/install_work_services.py \
  --zaturo-root /absolute/ZaturoBot \
  --youtube-root /absolute/YoutubeBot \
  --enrollment /absolute/private/work-services.enrollment.json \
  --enable-scheduling
```

The default is a read-only plan. Review the pinned revisions, `work-pc` identity,
capabilities and flags, then repeat with `--apply`. `--enable-scheduling` grants the
bounded typed scheduled-batch service; **it does not create a video job**. Omitting it
preserves the existing scheduling grant; a worker without that grant stays read-only
for scheduling. Existing generation, USB, private-view and other reviewed grants are
also retained rather than reset by an omitted flag. Free chat is not upload authorization.

The normal apply sequence prepares binaries/dependencies while the old worker remains
available. It builds `zaturo-worker` and `zaturo-desktop` locally using
`cargo build --release --locked`. A fresh YoutubeBot `.venv` installs an official CPU
PyTorch wheel and the pinned `requirements-desktop.txt`; see the official
[PyTorch Linux installation selector](https://pytorch.org/get-started/locally/) for
GPU-specific alternatives. No model weights are downloaded and optional Qwen/other
engines are not silently changed. An existing virtualenv is preserved: verify its
packages and use `--skip-python-deps` rather than modifying it during active work.
The extension is rebuilt with the pinned `scripts/build-chrome.py`, without opening
a browser. Repeated matching environment/extension preparations are skipped.

Only after preparation and a second source/config check does the updater stop
`zaturo-worker.service`. The hub SSH tunnel and file identity remain configured.
It creates a consistent SQLite backup via SQLite's backup API and verifies integrity,
then privately backs up the existing configuration, worker environment, unit and
binaries. It atomically publishes the new binaries/configuration and starts the worker,
unless `--no-start` was supplied. Starting the new worker migrates the existing journal
in place to schema 8; the updater itself never replaces/deletes that journal. Existing
finished tasks and uncertain-operation fences must remain intact. Check the actual
hub/worker handshake after start: systemd active alone is not proof of hub connectivity.

The original worker unit/resource settings are preserved if its executable/config
paths match the reviewed layout. Heavy YouTube actors use the independent user-systemd
units implemented by YoutubeBot: 12 GiB MemoryHigh, 16 GiB MemoryMax, CPUQuota 200%,
TasksMax 256 and a 12-hour deadline. They do not run inside the small worker's 512 MiB
cgroup. This helper does not start a media actor or weaken that boundary.

## Optional USB/private-screen registration

Discover the attached USB devices locally with `adb devices -l`. Discovery is not a
grant, and the installer never chooses a device automatically. To register the intended
USB device, add `--android-serial EXACT_USB_SERIAL`. The default logical ID is
`owner-phone`; use `--android-device-id` only for an explicitly chosen different ID.
A changed existing serial is refused rather than silently rebound. Network/emulator
selectors are not accepted.

Add `--enable-private-view` **together with the explicit serial** only for the owner's
sensitive-screen grant. This enables the bounded private service; each actual screenshot
still requires separate native-client confirmation. The permitted screen must be on
an unlocked phone. No unlock/PIN bypass, OCR, LLM, background SMS harvesting, plaintext
feed history or screenshots saved to disk are installed. A disconnected/unauthorized
phone remains unavailable. Linux and Android now have private viewers. Android generates its own age identity
inside its Keystore-protected settings; it never imports another PC's private identity.
Add `--enable-messages` with the explicit serial to grant fixed messaging-app opening.
It never grants SMS sending, OCR, account sign-in or screen-lock bypass. Verified steps
are recorded as metadata in `workspace_root/android-instructions/DEVICE.json` on that PC.
Add `--enable-generation` for generation-only YoutubeBot work. This does not grant upload
or scheduling. Existing scheduling grants are preserved when present.
The updater also installs the compact local MCP adapter/profile; use its printed
credential-free registration command and [MCP instructions](../mcp/README.md).

## Optional computer and stored-price capabilities

The 0.6 updater recognizes all 13 current typed capabilities, including
`computer.action.v1`, `computer.private.v1`, `price.search.v1` and an already configured
`computer.agent.v1`. It does not add a computer planner or copy a provider key to the
work PC. The distinct home worker owns the bounded Personal/Go planner under explicit
allow_personal_go opt-in, the same exclusive DeepSeek model and durable budgets. The old
Console402 is historical, not a current Go requirement. Observe the actual Go route and
subscription billing separately; keep Extra Usage off and never add Console credits or
copy a provider key merely to enable work-PC actions.
The hub must support schema9 computer coordination; the work journal remains schema8.
See [computer contracts](../../docs/COMPUTER_USE.md) and
[social contracts](../../docs/SOCIAL_MANAGEMENT.md) before enabling a backend.

Every base update installs the four reviewed fixed Python helpers in
`~/.local/lib/zaturobot/computer`, a mode0700 directory, without enabling a new computer
grant. The separate `deploy/computer/install.py` can create disabled private Linux/browser
profiles; neither helper selects applications, windows, origins, CDP sessions or accounts.
It does not install OS packages, start a new backend service, enable input/capture or
perform login. Preserve existing profiles and local state. Actual GUI/AT-SPI/compositor
readiness and exact runtime artifact/license/package-tree hashes must be verified on
the work PC according to the computer runbook. Private PC capture is native-X11-only;
the upstream Wayland/portal capture routes can write temporary PNG files and are refused.
Semantic Wayland observation/input remains a separate capability-dependent check.

An explicit `--computer-config /absolute/private/computer.toml` supplies a standalone
mode0600 `ComputerConfig` file, as shown in
[the example](../../config/computer.example.toml). Its top-level fields are only absolute
`python`, `scripts_root`, optional `linux_profile`, `browser_profile`, `bitlupa_project`,
`bitlupa_database`, `social_profile` and the optional `capture_target` table. Do not wrap
it in another `[computer]` table or put keys, environment contents, browser endpoints or
model-supplied executables in it. Backend profile pointers must identify existing private
files. The updater overrides only `scripts_root` with its stable reviewed installation
directory and preserves every backend profile's contents and independent grants.

Add only the reviewed grants to the normal plan, then repeat with `--apply`:

```bash
python3 /absolute/ZaturoBot/deploy/services/install_work_services.py \
  --zaturo-root /absolute/ZaturoBot \
  --youtube-root /absolute/YoutubeBot \
  --enrollment /absolute/private/work-services.enrollment.json \
  --computer-config /absolute/private/computer.toml \
  --enable-computer-actions \
  --enable-price-search
```

`--enable-computer-actions` advertises bounded actions and requires at least one reviewed
Linux/browser/social/BitLupa backend path. It does not grant backend input or account
actions. `--enable-price-search` additionally requires the actual reviewed BitLupa project;
the current source is schema/configuration only, with no collector or enabled collection
source. Missing data and the untracked base POCO X7 remain explicit blockers; X7 Pro
must not be substituted. Existing BitLupa source/data stays read-only.

`--enable-computer-capture` separately requires `linux_profile` and an exact locally
observed `capture_target` with precisely `window_id`, `pid` and `app_id`. The Linux profile
must independently grant capture; no whole-desktop fallback or automatically selected
target exists. Images use the ephemeral encrypted native viewer and never model context,
normal feed history, disk or MCP. Changed window/PID bindings require deliberate local
refresh. Each actual view remains an explicit owner request with expiry/focus clearing.

Each new flag requires the explicit private configuration or a validated existing worker
`computer` profile. A base update preserves existing `computer`,
`computer_models_config` and capability grants, while normalizing only the helper path.
The planner profile is not created, read into another file or enabled by this updater.
Absent flags do not revoke previous grants or expand disabled backend permissions.
Invalid/aliased configuration, symlinks, broad permissions or missing grant prerequisites
fail before replacing the worker configuration. No account credentials are copied.

Custom input still waits for the exact native-owner confirmation of the observed target
and snapshot. `/aprovar UUID` or `/recusar UUID` uses the native `computer_confirm` action;
ordinary assistant/MCP approval text is inert. Shared tasks/files are never autoexecuted.
Do not test readiness by publishing, uploading, replying to comments, bypassing login or
retrying an uncertain action. Optional Postiz supports only its six documented platforms
and reviewed create/schedule/draft flow, not universal account/comment management.

## Installed paths and preserved state

The stable binaries are `~/.local/lib/zaturobot/zaturo-worker` and `zaturo-desktop`.
An existing `~/.local/bin/zaturo` wrapper is preserved; if absent, a compatible wrapper
sources only the existing worker environment. `~/.local/bin/zaturo-desktop` starts the
native client with the private desktop configuration. A user-local desktop entry and
brand SVG icon are installed. Add `~/.local/bin` to the normal user PATH if necessary.
The desktop entry itself uses an absolute executable path.

`~/.config/zaturobot/app.env` contains only the new scoped app token, mode 0600;
`desktop.toml` targets `work-pc` and reuses the registered inbox/age identity. The
configuration directory remains 0700. Existing transfer/SSH/worker/provider credentials
are not copied into the application environment. A preexisting `app.env` containing
anything other than app-token assignments/comments causes rejection. An existing
desktop profile with different transfer paths also requires deliberate review.

Backups and the private install log/report live under `~/.local/state/zaturobot/`.
Each backup has a checked `worker.db`, numbered 0600 files and a manifest mapping their
original paths, modes and hashes. SSH keys, the age identity, provider files and media
inbox are not duplicated into this backup. No backup or report is sent to GitHub.

If installation fails after quiescing, the worker stays stopped and the backup retains
`UPDATE-INCOMPLETE.txt`. Inspect both journal schemas before a manual rollback. A new
worker might already have migrated the database: **do not restore an older binary over
a newer journal**, delete operation fences, or replace the current DB automatically.
Retain the failed update's live journal for evidence and recover with the compatible
source/binary. Preparation failure occurs before the old worker is stopped.

## Verification and remaining real acceptance

After a normal apply, verify `systemctl --user status zaturo-worker.service`, the
unchanged hub tunnel, the actual hub's work-PC version/capabilities and a bounded
diagnostic. Open `zaturo-desktop`; verify today's feed, filters and the explicit
YouTube-profile/device listing buttons. Keep account/video output private. The updater
never performs real Google login, account binding, video generation or uploads.

Run the YoutubeBot account window locally using its `.venv/bin/python -m tools.ytbot
accounts ui`; sign in interactively and verify the exact YouTube channel ID before any
explicit scheduled batch. A restored browser profile may require fresh authentication
because the OS keyring differs. Restore private history/media using the reviewed
separate helper; virtualenvs, node_modules and model caches are rebuilt, not overlaid.
GPU/model engines, TTS voices and optional large weights retain their original setup.
Physical Android/USB/private-screen acceptance requires the actual authorized phone.
The Android APK still requires installation/import on a real device and foreground
SSH/Keystore acceptance; a successful APK build is not a device test.

For an explicitly verified prebuilt pair, use `--skip-build` plus both
`--worker-binary`, `--desktop-binary`, `--worker-sha256`, `--desktop-sha256` values from
the trusted build handoff. Merely claiming a binary version is insufficient. Source
pin/cleanliness checks still apply. Do not invent hashes or use an unrelated build.

Offline simulation uses a separate, private fixture home with an existing synthetic
profile/database, `--home /tmp/private-fixture --no-start --apply`, explicit prebuilt
hashes and an already prepared fixture virtualenv with `--skip-python-deps`. It never
contacts systemd for the real user. A process using that alternate worker config is
rejected. Do not use this mode to skip quiescing an actual live profile.

```bash
python3 -m unittest discover -s deploy/services -p 'test_*.py' -v
```

The updater tests use synthetic clean repositories/credentials/journals, never real
accounts or phones. They verify dry-run silence, backup consistency including committed
WAL records, stop-before-backup-before-replacement ordering, evidence/keys/token
preservation, explicit capability grants, private file modes, source/hash/symlink
rejection, idempotence and interrupted-update recovery. The offline fixture intentionally
does not start a worker or pretend to test the production schema migration/SSH/Google.


## Additive0.7 installed execution

Read [the installed OpenCode contract](../../docs/OPENCODE_EXECUTION.md). Deploy the
compatible hub before advertising `assistant.route.v1`/`youtube.opencode.v1`. Supply an
explicit nonsecret `--opencode-config`, `--enable-generation --enable-opencode`; no key
field exists. The actual installed binary/version/hash and protected Go integration stay
local. The same worker budget/journal fences are reused. Computer configuration separately
accepts `wayland_portal_capture=true` without a fixed X11 target; native portal consent is
still mandatory. New flags never imply native action approval or account login.

The owner directed hub administration and existing-signature APK build to the main PC.
Use the revision-pinned shared handoff. Preserve this work PC's forwarding key, worker
journal, independent media units, enrolled serial, app data and all account profiles.
Until the hub update is verified, keep the two new capabilities unadvertised. Existing
computer/Android capabilities can be validated independently after the local update.
