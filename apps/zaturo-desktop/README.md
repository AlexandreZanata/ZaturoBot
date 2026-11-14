# Linux desktop client

Native Rust + Slint 1.18.1, Linux only. The official Slint interpreter loads embedded,
build-validated `.slint` source so generated FFI does not weaken the workspace's
`unsafe_code = forbid` rule. The software renderer and pinned winit integration provide
native display, icon and focus/visibility handling. No browser/webview is embedded.

## Build and connect

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
hub history, with bounded older-page loading. Messages record a request; arbitrary text
is not claimed as executed work. Explicit diagnostic, YouTube-profile and Android-device
buttons dispatch the supported typed actions. Completed task/file notices come from
the hub. Registered workers show their last contact, not an invented live counter.
The request queue is bounded and stale refresh requests are coalesced.

Non-sensitive photo cards use an artifact UUID and preview only a verified already-
received PNG in the private `inbox_root/UUID` folder. Symlink/traversal paths, multiple
files, files over 8 MiB and dimensions above 4096 are rejected before loading. Missing
content remains pending for the encrypted delivery client; no plaintext image endpoint
or arbitrary remote URL is introduced.

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
