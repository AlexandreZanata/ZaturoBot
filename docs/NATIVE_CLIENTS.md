# Native clients and activity

The owner authorized an initial Linux Rust/Slint desktop and native Android Kotlin app.
Both are working clients of the existing lightweight hub, not separate automation stacks.
Engineering artifacts remain English; interface copy defaults to Brazilian Portuguese.
Use the supplied fox SVG, gold `#E5B900`, black `#0B0B0D`, white and shared brand tokens.
The [branded design document](design/README.md) explains the visual and system boundaries.

The hub stores ordinary chat/activity in schema 5. A day filter plus fixed timezone offset
selects local midnight boundaries against UTC timestamps; default owner timezone is
America/Cuiaba. Search is a literal Unicode-lowercase substring, not executable SQL or a
model request. Pages are bounded; task completion/cancellation/failure and file delivery
produce stable deduplicated notices. All past terminal tasks seed notices on upgrade.
Historical data remains in SQLite when another day is selected; the main view defaults
to the current local day. Closing a client does not cancel an ordinary durable task.

Free text is stored honestly as a request. Explicit diagnostic, YouTube-profile and USB
inventory actions create typed tasks. Arbitrary text is not treated as permission to
execute a shell or an unsupported operation. The existing public-coding model route stays
separate; account documents and phone screens are never automatically sent to it.

## Authentication and transports

Each app has a distinct revocable bearer digest in a private hub `apps_file`, separate
from worker, owner-admin and model credentials. `can_submit` permits only the scoped
application routes, not owner task lifecycle/control APIs. `viewer_device` optionally
binds a desktop to its endpoint age identity; absent bindings cannot request private
screens. Removing/replacing a client entry and restarting the hub revokes it without
replacing a worker journal or SSH identity.

Desktop uses the installed pinned SSH tunnel (literal loopback HTTP) or a private HTTPS
origin. Android imports a separately protected enrollment JSON into Keystore-encrypted
storage and opens its own host-pinned restricted SSH connection on port 22, forwarding
only the private hub. It requires no new public API port or notebook router rule. Polling
and connection ownership follow visible app lifecycle. Literal-loopback HTTP disables
proxy routing. Credentials never enter URLs, logs, feed items, source or GitHub.

The initial Android app supports chat, daily history, service actions, worker status and
its private SSH transport. Normal photo cards reference encrypted file-transfer UUIDs;
the desktop can display a verified bounded PNG already received in its private inbox.
An unavailable local attachment is labeled pending; it is not fetched as plaintext from
the VPS. Android has no native age private-screen viewer in this slice and receives no
private-view binding. APK build and JVM transport tests are not real-device acceptance.

## Private desktop screen panel

Private Android views are separate from the ordinary feed. An explicit owner confirmation
creates one finite serial-bound request. A registered endpoint public key is selected by
the authenticated app binding, not supplied by screenshot content. The worker acquires
one server grant, checks every parameter, captures/watches an authorized USB device and
age-encrypts raw PNG bytes directly in memory. A screenshot does not become a file task.
The hub holds at most two encrypted frames in RAM, never SQLite or filesystem spool.
Frames expire within 60 seconds after upload; waiting time is watch plus 60 seconds,
at most 180. Owner reads consume the frame once. Private routes send `no-store`; cancel
revokes the RAM session and its pending task. Empty watches report `unchanged` normally.

The desktop decrypts in protected memory, bounds PNG dimensions/bytes, shows a dedicated
panel and clears it on expiry/focus loss/close. Owned plaintext buffers zeroize on drop;
OS/compositor render buffers are outside a Rust buffer-wipe guarantee. Fixed acceptance
metadata prevents arbitrary captions, OTP text or tiny image payloads entering task SQL.
There is no OCR, model processing, notification harvesting, automatic account sign-in or
lock/secure-window bypass. The messaging screen must already be accessible/unlocked;
a byte-level screen change is not proof that an SMS arrived. See
[Android sensitive views](ANDROID_SENSITIVE_SCREENS.md) for physical limitations.

## Entry points

- [Desktop source and installation](../apps/zaturo-desktop/README.md).
- [Android source, APK build and enrollment](../mobile/android/README.md).
- [Existing encrypted device transport](FILE_TRANSFERS.md).
- [YoutubeBot service](YOUTUBE_SERVICE.md).

REST: `GET /v1/feed`, `POST /v1/chat`, `GET /v1/services`, read-only `GET /v1/workers`
and metadata-only `GET /v1/artifacts/{uuid}` use application authentication. Private views
use `/v1/private-views` and worker-bound `/v1/device/private-views` routes. Owner-admin
and device endpoints retain their independent boundaries. Types in core/activity.rs,
core/service.rs and source validators are canonical; documentation is not another schema.
