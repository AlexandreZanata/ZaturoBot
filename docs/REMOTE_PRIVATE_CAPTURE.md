# Remote private page and SMS capture (0.8)

Registered integrations: `private-browser-capture` and `private-android-messages`.

## Owner workflow

Android Hub accepts `print https://example.com/` and opens a native confirmation with
an editable URL. Services → Print de página opens the same form. The worker opens the
URL in its separate bot-owned browser and returns one actual 1280×720 viewport PNG.
No desktop portal selection is needed for these bot-owned pages. General desktop
capture retains its existing native portal/window grant. A URL is required; arbitrary
existing desktop/browser tabs are not enumerated or silently attached.

Services → Print de SMS (or a chat request containing print and SMS/mensagens) asks
for the enrolled USB device and explicit native confirmation. After the fixed15-second
preparation, the worker opens the approved default messaging component, checks fresh
unlocked keyguard and matching focused window/application, captures once, then verifies
that Messages still has focus. A focus takeover drops the image. It never queries SMS
storage, OCRs text, sends SMS, selects contacts, unlocks or accepts USB consent.

Select “Estou usando o Hub nesse mesmo celular” only when the viewing app is on that
USB phone. With the separate local ReturnToHub grant, the worker returns only to
`dev.zaturo.bot/dev.zaturo.bot.MainActivity` using the fixed reorder-to-front flag;
the singleTask companion reuses its pending in-memory session. It never clicks a confirmation. The original
native confirmation also authorizes opening that pending image when the focused Hub
reconnects. Other viewers use Abrir captura preparada. No background image download;
expiry, consume-once, clear-on-focus/navigation loss and FLAG_SECURE remain enforced.

## Trust and transport

The native viewer posts source=browser/device_id=browser/url or source=android_messages
with prepared=true/return_to_hub. Hub validates allowed combinations before dispatch.
The exact URL exists only in the RAM-only PrivateCapture::Browser grant, not the durable
service task, normal chat, SQL, model prompt or feed. Grant Debug redacts it. Durable
BrowserPrivateView and AndroidMessagesPrivateView carry only opaque session/recipient
metadata and bounded options. New worker capabilities are browser.private.v1 and
android.messages.private.v1. They require a compatible0.8 Hub before advertisement.

Private browser rendering uses the already pinned Node and Playwright-core tree plus
an independently hash-verified Chrome executable. It launches its own private persistent
profile with Chromium sandbox enabled and a fixed viewport. No arbitrary evaluation,
input tool, screenshot path, trace or video is exposed. PNG uses a bounded stdout pipe,
then zeroizing worker memory → age encryption → RAM-only Hub → native viewer. The
existing Linux subreaper cleans the owned browser tree on EOF, deadline and cancellation.
The local operation journal stores a digest, never the URL or image. STOP in the private
capture state directory prevents the next launch and terminates a running guardian.

Only HTTP(S) public addresses on standard ports are accepted; credentials, nonweb
schemes, local/reserved DNS results and mixed public/private answers are rejected.
Each intercepted request/redirect and WebSocket is checked, service workers and downloads
are disabled. These application checks are not an OS/network sandbox and do not promise
network-level DNS-rebinding isolation. Chromium still executes ordinary page scripts.
HTTPS validation stays enabled. Pages requiring CAPTCHA, consent, unusual ports or login
may be blocked or display that page; no authentication/protection bypass is attempted.

The private browser profile is separate from semantic research, normal accounts and
YoutubeBot. It may retain its own cookies/site cache; protect it as private account data.
It never joins a semantic/model session. To authenticate, the owner may locally open that
same profile with the verified Chrome, complete login, then close it before capture.
No cookies or passwords are copied from other profiles. This release captures one
viewport, not an entire scrolling document. A screenshot is not proof of publication.

## Installation and update

1. On each PC, preserve the checkout, ignored data, enrollment, databases and current
   service configuration. Fetch feat/work-pc-real-execution into an isolated review
   worktree at the desired reviewed commit; do not reset/clean an existing checkout.
2. Run cargo xtask check --locked and cargo xtask secrets --locked. The repository CI
   also compiles/lints/tests Android without assembling, signing or uploading an APK.
3. On the already authorized main PC, build/deploy the compatible Hub using the existing
   private deployment context and consistent recovery snapshot. Preserve schema9,
   app scopes, recipient, restricted forwarding keys and all service identities.
4. The additive computer installer copies browser_private.py/.mjs and creates a DISABLED
   computer-private-browser.toml. Fill its verified Node/CLI/package-tree/Chrome pins,
   use a NEW0700 browser-private-capture directory with a0600 marker named
   .zaturobot-private-browser containing {"schema":1,"purpose":"zaturobot-private-page-capture"}.
   Give it a separate0700 state_dir. Review enabled=true, allow_capture=true,
   allow_input=false and timeout_seconds=19 locally. Set ComputerConfig.private_browser_profile
   to this private TOML. Never reuse the semantic research profile.
5. Work service installer flags: --enable-browser-capture; SMS requires the explicit
   existing --android-serial plus --enable-private-view --enable-messages
   --enable-messages-capture; --enable-return-to-hub is separate. Existing profiles and
   keys stay private/preserved. Advertise the new capabilities only after Hub rollout.
6. Main PC uses its EXISTING Android SDK/JDK and signing keystore. Verify the old and new
   public certificate match with apksigner, package dev.zaturo.bot, versionCode6/name0.8.0.
   Build the deliverable there and transfer through the encrypted device-file channel.
   Install with adb install -r after comparing hash/certificate; never uninstall, clear
   data, regenerate the key or reenroll just to update. A different phone needs its own
   separate scoped enrollment; do not clone another phone's private identity.
7. Confirm a real page and SMS image in the physical Android Hub. Record opaque task IDs
   and metadata-only displayed receipts; keep screenshots/URLs/SMS and all credentials
   out of GitHub. Compilation and synthetic renderer tests are not physical acceptance.

## Validation status

The owner confirmed the previous real Wayland PC image appeared in Android Hub. The
new0.8 URL/SMS paths require the compatible Hub and same-signature APK deployment plus
physical native acceptance; source implementation does not establish those outcomes.
Offline tests cover URL/address rejection, grants, typed ADB focus/lock checks, image
zeroization, binary pipes and preservation. A local synthetic page rendered successfully
with the actual installed sandboxed Chrome (PNG1280×720,37012bytes); no private page or
screen was opened and no screenshot file was written. Real AI/OpenCode/media acceptance
remains independently tracked by the original operation fences.
