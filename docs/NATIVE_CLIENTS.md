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

## Shared native presentation and chat navigation

The Android 0.4.1 presentation uses compact bottom navigation; the Linux client adapts
the same destinations to a desktop sidebar and wider layout. Both use flat, subtly
outlined buttons without shadows, right-aligned gold owner bubbles, left-aligned dark
assistant/system bubbles, and locally bundled official Material Symbols Rounded icons.
[Android icon provenance](../mobile/android/docs/ICONS.md) and
[desktop SVG provenance](../apps/zaturo-desktop/assets/material-symbols-rounded/README.md)
record the pinned primary assets and their license. This visual vocabulary does not add
messenger accounts, calls, arbitrary attachment execution, or any new automation grant.

The common navigation contract is explicit:

- Opening or reopening Hub shows the bottom of the currently selected day/type/search
  view. A new visit does not restore an unread-message position over this owner preference.
- An accepted own send switches to today's local date, **Tudo**, and empty search so an
  old day or restrictive filter cannot hide the accepted message. It requests a fresh
  first page and a bottom jump after the actual content/viewport measurement, including
  when the refreshed rows compare equal. A failed or unconfirmed request is not an
  accepted send and keeps its existing idempotency/reconciliation boundary.
- Automatic positioning never requests composer focus or summons the keyboard. Incoming
  data while the owner reads older history retains the reading anchor; unchanged polling
  does not rebuild message widgets or move focus. Following new incoming rows is limited
  to a view already following its bottom or a pending explicit navigation/send intent.
- Older pages prepend chronologically while retaining the visible anchor. The client
  deduplicates stable IDs, filters sensitive items, and retains at most 200 ordinary
  rows. Pagination offsets advance by raw server page counts, including any filtered or
  duplicate rows, rather than by the visible-row count.
- Draft and current search text stay in RAM across tabs. An accepted send intentionally
  clears the search as part of its view reset. Neither tab changes nor this presentation
  work export plaintext drafts to Android saved state or a new desktop draft file.

Telegram Desktop's official
[history widget source](https://github.com/telegramdesktop/tdesktop/blob/dev/Telegram/SourceFiles/history/history_widget.cpp)
is a behavioral reference: it distinguishes positioning at the end, positioning at
unread content, and restoring a saved scroll anchor. That observation guides separate
navigation intents here; ZaturoBot's owner explicitly chooses the end on every Hub open.
No Telegram code was copied and no complete behavior or feature parity is claimed.

These changes preserve authentication, supported actions, encrypted connection storage,
ordinary/sensitive-feed boundaries, the model route, and the private-view lifecycle.

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
