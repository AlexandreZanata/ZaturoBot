# Linux desktop client

Native Rust + Slint 1.18.1, Linux only. The official Slint interpreter loads embedded,
build-validated `.slint` source so generated FFI does not weaken the workspace's
`unsafe_code = forbid` rule. The software renderer and pinned winit integration provide
native display, icon and focus/visibility handling. No browser/webview is embedded.
The flat sidebar/controls, official local Material Symbols and gold/right owner bubbles
share the Android visual language while keeping a desktop layout.

## Build and connect

On Ubuntu/Debian, install the native compiler/linker and Fontconfig development metadata
before the pinned Rust build. The Slint font-discovery dependency uses `pkg-config` even
when rendering uses the software backend:

```bash
sudo apt-get install build-essential pkg-config libfontconfig1-dev
```

An actual window also needs a running X11/Wayland session and its normal XKB/font runtime
libraries. CI installs the same Fontconfig build prerequisite and runs noninteractive
tests; a successful headless build is not display/hardware acceptance.

```bash
cargo build --release -p zaturo-desktop --locked
target/release/zaturo-desktop
```

The first-run settings page accepts a private HTTPS origin or literal-loopback HTTP
inside the existing SSH tunnel, a scoped app token and target worker ID. Prefer the
new application credential rather than copying an owner/admin/AI credential. The
private default configuration is `~/.config/zaturobot/desktop.toml`; the default
credential file is `~/.config/zaturobot/app.env`. Both are 0600 in a 0700 directory.
A supplied token is saved only in a private `desktop.env`; existing owner/worker keys
and journals are not overwritten. An environment `ZATURO_APP_TOKEN` (or legacy
`ZATURO_OWNER_TOKEN`) is also supported. Never put tokens in argv or source control.

See `config.example.toml`; pass `--config /absolute/private/desktop.toml` for another
profile. `--page private` opens the private-phone panel. `--quit-after-seconds N`
supports bounded headless UI smoke tests. The optional desktop launcher template is
`packaging/zaturobot.desktop`; install it and the supplied brand SVG under the user's
standard application/icon paths with the reviewed release binary.

## Actual behavior

The hub starts with the current local day. Date, type and text filters query persistent
hub history, with bounded older-page loading. Rows render oldest to newest. Opening Hub
always scrolls to the newest row after layout; a confirmed send returns to today with
all kinds and an empty search, then displays the fresh page at the bottom. Failed or
uncertain writes are not automatically retried. Messages record a request; arbitrary
text is not claimed as executed work. Explicit diagnostic, YouTube-profile and Android-device
buttons dispatch the supported typed actions. Completed task/file notices come from
the hub. Registered workers show their last contact, not an invented live counter.
The request queue is bounded and stale refresh requests are coalesced. A write may queue
behind a read refresh; another write and generation-changing filter/connection changes
are blocked until its acknowledgement. Credentials and authorization stay in the same
bounded HTTP adapter.

The in-memory timeline deduplicates up to 200 nonsensitive rows and keeps raw server
paging offsets even when the HTTP adapter removes a sensitive row. Newest-page refreshes
preserve overlapping older pages; an unknown gap replaces the cache instead of implying
continuous history. Older pages prepend above the conversation while preserving the
visible content anchor, including removal of the older-page button. Unchanged responses
do not rebuild message views. Periodic reads pause while the owner reads above the bottom
or edits an unapplied day/search filter. Explicit filter changes reset the relevant cache.

Viewport intent is applied by revision-guarded Rust one-shot timers after positive,
stable layout metrics. Native scrollbar position is bridged separately from the inherited
Slint property alias, so wheel/thumb movement and the exposed state agree. User scrolling
cancels old pending work; late geometry changes still retarget a tail-following view.

Non-sensitive photo cards use an artifact UUID and preview only a verified already-
received PNG in the private `inbox_root/UUID` folder. Symlink/traversal paths, multiple
files, files over 8 MiB and dimensions above 4096 are rejected before loading. Missing
content remains pending for the encrypted delivery client; no plaintext image endpoint
or arbitrary remote URL is introduced.


## Icon distribution

The 16 official Material Symbols Rounded SVGs are compiled into the binary alongside the
brand SVG; the installed app does not depend on a source-tree path or runtime download.
See [the pinned icon provenance](assets/material-symbols-rounded/README.md). When shipping
or installing a standalone binary, include `LICENSE.txt` and `NOTICE.txt` from that asset
directory. Also retain `provenance.json` so the exact upstream revision and hashes remain
reviewable. The user installer places these files at
`~/.local/share/zaturobot/licenses/material-symbols-rounded/`. This license applies to
the Google icon subset, not the owner's separate brand mark or the whole project.

## Private phone screen

This panel is separate from chat/feed/history. The owner must select the registered
phone and explicitly confirm each capture. A visual-change wait can be 0–120 seconds;
it does not identify/read OTPs. The unlocked phone's SMS application must already be
open. Android secure windows and lock screens are respected.

The worker encrypts a raw PNG with standard age to this desktop's registered file
identity. The VPS holds ciphertext in RAM briefly; the viewer fetches once, decrypts
through the transfer crate directly into zeroizing memory, and decodes a bounded PNG.
`transfer_identity` defaults to `~/.config/zaturobot/transfer.identity`. No plaintext
file, OCR, model call, feed artifact, clipboard or tool log is produced. The frame is
cleared at the earlier server deadline/60-second local lifetime, tab close, focus loss,
window hiding/minimizing, or explicit close; close requests hub revocation (relay TTL still bounds a lost connection). No-change,
cancelled and unavailable-device states produce no image. The owned plaintext byte
buffers are zeroized; renderer/OS copies are released/hidden rather than promising a
complete operating-system RAM erasure. Real-device acceptance remains opt-in.

## Checks

```bash
cargo clippy -p zaturo-desktop --all-targets -- -D warnings
cargo test -p zaturo-desktop --locked
```

Focused tests exercise real loopback authenticated HTTP/query encoding, sensitive-item
exclusion, private settings, no-follow/bounded PNG paths and in-memory private decoding.
Visual checks run in a separate Xvfb display with synthetic feed data; they never operate
the owner's live desktop or access a real SMS inbox. The root quality command remains
`cargo xtask check --locked`.


The optional `private_view_smoke` example runs only inside a separate Xvfb display. It
uses a generated identity and synthetic PNG, a loopback fixture with a shortened server
TTL, one-time ciphertext fetch and DELETE cleanup. It asserts that pixels were shown,
then cleared by the actual earlier server deadline, with no plaintext image file:

```bash
env -u WAYLAND_DISPLAY xvfb-run -a cargo run -p zaturo-desktop \
  --example private_view_smoke --locked
```

The synthetic viewport example checks actual rendered geometry without a hub, token,
account or phone: initial/explicit-entry/send tail intent, unchanged responses, reading
above the bottom, older-page anchoring, and chronological model order. Use a separate
Xvfb display:

```bash
env -u WAYLAND_DISPLAY xvfb-run -a cargo run -p zaturo-desktop \
  --example chat_scroll_smoke --locked --offline
```

A separate isolated local-HTTP visual fixture exercised the real binary: mouse-wheel
history reading pauses polling, explicit Hub entry returns to the latest messages, and
one confirmed send from an older-day view resets day/kind/search and exposes the new
right-aligned bubble. These are the requested chat behaviors; this version does not claim
complete Telegram/WhatsApp feature parity.
