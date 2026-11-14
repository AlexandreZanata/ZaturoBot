# USB Android integration

The owner authorized a base for a phone connected by USB to the work notebook, plus
future owner-only viewing of incoming SMS screens. The supplied Android architecture
attachment is design input, not permission to activate every proposed tool or bypass
Android controls. This implementation provides typed discovery/status, bounded screen primitives, revocable
leases and an endpoint-encrypted private desktop viewer. It does not claim that a physical
phone has been registered or that actual SMS arrival has been detected.

## Components and current boundary

Pure registration, state, capability, status and finite-watch contracts live in
`crates/core/src/android.rs`. `zaturo-android` owns fixed ADB invocations, parsing,
subprocess lifecycle and local session arbitration. The worker composition owns its
private executable/USB registration configuration and invocation. Hub/domain services
own authenticated requests, task results and safe feed metadata. Desktop/mobile clients
must not receive a claimed successful phone operation without typed worker evidence.

The VPS coordinates metadata. It does not run ADB, scrcpy, an emulator or video capture.
ADB uses the notebook's loopback server on port 5037. Captures are returned only as a
`SensitiveFrame` in protected process memory, with no serialization, debug formatter,
file writer or model adapter. Normal VPS store-and-forward files are not a sensitive
screen channel. The dedicated private-view path encrypts on the notebook to the viewer's
registered age public recipient, holds only a bounded short-lived cipher in hub RAM and
consumes it once in the authenticated desktop viewer. Images never enter SQL or the normal
file spool. Local `allow_sensitive_views`, exact USB registration, sensitive capability and
per-request owner confirmation are all required. Configuration remains disabled until those
grants and real hardware are prepared. See [sensitive screens](ANDROID_SENSITIVE_SCREENS.md).

`AdbClient::discover(now)` parses bounded `adb devices -l` metadata, including unauthorized,
offline, missing-udev and multiple-device cases. `inspect(registration, now)` reads four
fixed properties only after matching a ready USB serial: manufacturer, model, Android
release and API level. A configured serial never silently falls back to another phone.
Absent phones report `DISCONNECTED`; offline phones report `CONNECTING`; required RSA
consent reports `UNAUTHORIZED`; missing USB permissions report `ERROR` with a distinct
permission status. Network and emulator observations are visible but cannot satisfy a
USB control registration. Discovery alone grants no screen or interactive permission.

## Physical setup and configuration

Install the official Android Platform Tools or the distribution's reviewed `adb` package
on the notebook. The owner enables Developer Options/USB debugging, connects a data
cable, unlocks the device and accepts the notebook's RSA authorization dialog. The bot
cannot do those physical steps remotely. A reboot may require another physical unlock.
Use the exact serial from discovery, not a USB-port position or an assumed first device.

The worker's optional Android profile should contain an absolute trusted executable and
one or more independent registrations. For example, the registration portion is:

```toml
[[android.devices]]
device_id = "owner-phone"
adb_serial = "USB_SERIAL_FROM_THE_OWNER_DEVICE"
display_name = "Owner phone"
capabilities = ["device_status"]
```

Keep actual serials and host configuration private. Enabling `sensitive_screen_view` is
a separate explicit owner grant and still does not activate a remote viewer. There are
at most eight registered phones. The application must reject duplicate IDs/serials.
Binding and capability validation are canonical Rust rules, not copied documentation
schemas. Do not copy ADB private keys into a project or the VPS.

## Existing project assessment

The read-only [CelularLinuxADB](https://github.com/AlexandreZanata/CelularLinuxADB)
reference (`runcell` 1.0.1, inspected 2026-11-13) provides useful USB setup, authorization
state diagnostics, explicit `-s` selection and scrcpy launch controls. Its scripts were
not changed or imported. Its interactive text selection, unrestricted scrcpy passthrough,
Wi-Fi enablement, ambient configuration and detached window lifetime are inappropriate
as the model-facing structured adapter boundary. ZaturoBot therefore invokes fixed ADB
operations directly and reuses the operational setup/state conventions. The owner may
continue using `run cell` independently for local manual mirroring.

## Verification and remaining work

CI uses synthetic device lists, fixed fake ADB executables and tiny synthetic PNG data.
It covers identity mismatch, unauthorized/offline/multiple devices, timeout, output caps,
opaque errors, cancellation, owner takeover, lease expiry and image-change observation.
No test contacts a real inbox, uses a real OTP or starts a real ADB server. Real hardware
acceptance requires an explicitly configured serial on the work notebook.

Next: run discovery/status on the work notebook, verify physical RSA consent, register the
phone, and validate the implemented ephemeral viewer against an owner-selected nonsecret
screen before enabling real sensitive capture/watch. Navigation, SMS package discovery, semantic UI Automator operations,
continuous mirroring, app installation and instrumentation remain separately scoped.
The private screenshot viewer and finite visual-change watch are not evidence that
interactive control, notifications or these later phases exist.

Official references: [ADB](https://developer.android.com/tools/adb),
[hardware authorization](https://developer.android.com/studio/run/device),
[scrcpy connection](https://github.com/Genymobile/scrcpy/blob/master/doc/connection.md).
