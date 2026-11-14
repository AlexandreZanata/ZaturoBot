# ZaturoBot Android companion

Initial native Kotlin Android app (API 26+), not a browser wrapper. It uses the supplied
wolf SVG converted into Android vector/adaptive icons and the black/yellow brand palette.
The hub shows today's activity by default, date/type/search filters and saved messages.
Explicit diagnostic, YouTube-profile and Android-device requests use the existing
worker task boundary; arbitrary chat text is recorded without claiming execution.
The conversation filter sends the hub's `chat` kind; **Serviços** sends `service`,
covering YoutubeBot and Android service activity. Labels and wire values share one catalog.

## Mobile presentation

Version 0.4.1 uses an aligned brand header, compact day/type controls, an expandable
history search and a bottom bar for Hub, Serviços, Reconectar and Ajustes. The composer
and conversation occupy the main space; diagnostic execution lives on the Services page.
Buttons use subtle outlines, native press feedback and no elevation. Official Material
Symbols Rounded icons are bundled as vectors with [license/provenance](docs/ICONS.md).

The API remains newest-first; the bounded timeline renders oldest-first with new replies
at the bottom. Owner bubbles align right in brand gold, assistant/system bubbles left
on a dark surface. Opening or reopening Hub goes to the end of the currently selected
view. An accepted own send returns to today's local date, **Tudo**, and empty search,
then follows the fresh page to its actual measured bottom even if its data is unchanged.
This explicit navigation does not take focus from the composer or open the keyboard.

Incoming updates preserve the reading anchor when the owner is browsing older history.
Older pages prepend above that anchor; deduplication and sensitive-item filtering do not
change raw server page counts used for the next offset. At most 200 ordinary rows are
retained. Unchanged background polling avoids rebuilding messages or moving focus.
Drafts and the current search stay in process memory across tabs; an accepted send
intentionally resets the search as described above. Automatic text state saving remains
disabled. System/keyboard insets keep controls reachable; the bottom bar temporarily
yields space to the keyboard. The physical app retains `FLAG_SECURE`.

The 0.4.1 UI was visually checked in a separate network-disabled synthetic preview,
including keyboard, services/settings and 1.3 font scaling. Owner-authorized Android
16/API 36 acceptance then verified Hub re-entry at the latest message, passive older
reading across polling, one normal chat send from an older position, and composer/caret
stability with the keyboard. The existing app identity and encrypted enrollment survived
the update. No protected phone screenshots or additional Android permissions were used.

The same [chat-navigation contract](../../docs/NATIVE_CLIENTS.md) applies to the Linux
client, with desktop sidebar navigation and the same official icon/bubble vocabulary.
The implementation preserves existing API, authentication, model and private-view limits;
it does not add full messenger features or import another application's implementation.

## Build

Install JDK 17+ and Android SDK platform/build-tools 36, then set an ignored
`local.properties` with `sdk.dir=/absolute/path/to/Sdk` (or the usual Android SDK env).
Gradle wrapper 9.9.0 and Android Gradle plugin 9.5.1 are pinned. Kotlin support is built
into this AGP version.

```bash
./gradlew :app:assembleDebug :app:lintDebug :app:testDebugUnitTest
```

The debug APK is `app/build/outputs/apk/debug/app-debug.apk`, an ignored build artifact.
No account/token/key is embedded. Do not commit APKs, signing keys, enrollment bundles,
SDK paths or build caches. The initial APK uses debug signing for private testing;
release distribution/signing is a separate owner-controlled step.

## Private connection

In Settings, choose **Import private connection** and select the owner-issued JSON
file. It must contain schema 1, client_id, ssh_host, ssh_port 22, ssh_user `zaturo-link`,
ssh_private_key_b64, host_key_sha256, app_token and target_worker. Never copy a VPS admin
key, another PC's key/journal or an AI credential. The import is encrypted using
AES-256-GCM with an Android Keystore key; backups and task screenshots are disabled.
The SSH private key is parsed in memory, not written as a plaintext key file.

SSHJ 0.41.2 provides public-key-only SSH. Host verification hashes the actual SSH wire
public key with SHA-256 and compares in constant time; Java X509 encoding is deliberately
not used. A separate Bouncy Castle provider alias supplies SSH cryptography without
replacing Android's platform provider. Logging is NOP and errors never include secrets.
The local forward binds only `127.0.0.1:17879` and targets only VPS `127.0.0.1:7878`.
No shell, SFTP, agent, remote forward or public ADB interface is used. There are four
bounded reconnect attempts with backoff. Transport and polling stop when the app leaves
the foreground. Reopen/Reconectar to establish a new session.

A separately configured HTTPS origin or existing loopback tunnel can also be used.
Public plaintext HTTP, URL credentials/path/query/fragment, redirects and HTTP proxies
for the loopback bearer are rejected/disabled. Android's normal TLS validation remains
active. As an optional USB-only test, `adb -s EXPLICIT_SERIAL reverse tcp:17878 tcp:17878`
can connect a phone to the notebook's already authenticated tunnel; this is not required
when the app-owned restricted SSH connection is configured.

## Privacy and current limits

No READ_SMS, notification-listener, accessibility or background SMS service is declared.
The Linux worker owns authorized Android operations. Ordinary feed drops sensitive
items. This initial mobile build shows attachment metadata; it does not decrypt age file
attachments or provide the dedicated sensitive-screen viewer. That viewer is initially
Linux-desktop-only; no home decryption identity is copied to Android to bypass this limit.
Messages use stable idempotency keys across an uncertain retry. No raw server error,
authentication code, SSH enrollment or token is displayed in diagnostics.

JVM tests cover origin/token validation, canonical SSH wire pins, wrong key/X509 denial,
and an explicit opt-in restricted live tunnel smoke. CI never contacts a production host.
To run the owner-authorized live test, supply only the path of the protected mobile
bundle through `ZATURO_ANDROID_SSH_TEST_BUNDLE`; no secret appears in argv/output. It
checks hub health and rejects a wrong host pin before authentication. It does not read
SMS, change accounts or operate an actual device.

An owner-authorized physical USB acceptance on 2026-11-14 used Android 16/API 36. The
dedicated connection was imported using app-private staging and verified through the
production Android Keystore code. The normal APK was restored with its data preserved;
the temporary import file and test instrumentation were removed. The real phone
synchronized its feed through its own pinned SSH transport, exercised conversation and
service filters, and submitted a diagnostic that completed on `work-pc` with a visible
history notice. Returning from the background reconnected successfully. No personal
phone messages or protected screenshots were accessed.
This is evidence for that opted-in device, not every Android version/vendor. Existing
work-PC installation is preserved; do not re-enroll or remove its journal to install
the app. Android Lint passes with non-fatal warnings for the supplied detailed vector,
initial Portuguese inline copy, conservative SDK target and unused third-party helpers;
none enables permissive TLS or suppresses errors.
