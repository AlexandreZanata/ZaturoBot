# Owner-only sensitive Android screens

An SMS verification-code screenshot is sensitive account material. This feature's
current implementation includes in-memory serial-bound capture, finite screenshot-change
watch and a dedicated endpoint-encrypted desktop viewer. It still requires physical USB
setup, local per-device grants and an explicit owner confirmation for each request.
The general file relay persists ciphertext and delivery metadata for up to its configured
retention; it is therefore not used for sensitive SMS screenshots.

## Required owner workflow

The desktop viewer authenticates the owner through a viewer-bound application credential,
identifies the exact registered phone and requests a short-lived sensitive-screen grant. The owner prepares/unlocks the
permitted screen. The worker atomically claims that grant, verifies its own ID, phone, recipient and duration,
acquires an owner control lease and captures only the approved view. It encrypts to the
registered viewer recipient with standard age. The hub temporarily forwards that cipher
through an authenticated RAM-only session, not a disk-backed artifact or general transfer. Only nonsensitive operation metadata enters ordinary task/feed history. The
Revocation or disconnection prevents pending retrieval. Once displayed, the viewer clears
the image through local dismissal/focus/navigation controls and the absolute expiry;
it does not continuously observe remote revocation after consuming the one-time frame.
The worker releases the lease and erases unreturned image buffers. No sensitive content
is sent to the configured coding model or another provider.

Current local APIs enforce a maximum five-minute lease, bounded command output and
owner-only screen permission. `AdbClient::capture` and `watch` return `SensitiveFrame`,
not a file or serializable task result. The application private-view request allows immediate capture or a 5–120 second visual
watch. A waiting grant lasts watch duration plus 60 seconds, at most 180 seconds; an
uploaded image expires within 60 seconds and can be consumed once. At most two sessions
are retained. Active cleanup runs every second. The hub never receives a private age
identity and cannot decrypt the image. `Cache-Control: no-store` applies to private-view
responses, and literal-loopback clients ignore ambient HTTP proxies.

The desktop respects the absolute server image deadline, caps image dimensions/decoded
allocation and clears the image on expiry, close, navigation, focus loss or minimization.
It does not create thumbnails or copy the image into normal feed state. A lost one-time
grant/download response requires a new explicit owner request. The application closes an
unchanged finite watch normally, without claiming that an SMS or code was detected.
A successful synthetic flow still is not a physical-phone acceptance test.

## Finite watch semantics

`AndroidWatchRequest` permits 5–300 seconds with 2–15 second sampling. The watch captures
one baseline, retains only its digest, then returns at most the first changed screenshot.
No change before the deadline is a normal empty result. Lease revocation cancels an
in-flight ADB process. Device loss/authorization loss is an error, not a reason to select
another device. The lease must cover the complete requested interval.

A changed PNG does not establish that a new SMS arrived or that it contains a code. A
clock, notification animation, app switch or PNG encoding change can trigger it. The
implementation does not read the SMS database, parse a notification dump, OCR pixels,
extract or use a code, alter the default SMS handler or install a background harvester.
It does not automatically open an assumed SMS package. Do not label a visual-change
result as `sms_received` or `otp_detected` in a client.

Android 15 and later restrict untrusted notification listeners' access to OTP-bearing
notifications; these protections must be respected, not disabled. See
[Android behavior changes](https://developer.android.com/about/versions/15/behavior-changes-all?hl=en).
Protected or locked screens may remain unavailable. SMS package/navigation and physical
unlock behavior must be verified on the owner's specific phone before claiming this
workflow works unattended.

## Remote acceptance gate

Validate every future change against owner-only access, worker/client identity binding, direct
end-to-end encryption, bounded memory/queue/backpressure, image expiry/erasure, no disk
persistence on the VPS or clients, no code in logs/feed/model context, and revocation
while a frame is in flight. Use synthetic SMS screens in automated tests and an explicit
nonsecret test screen on opted-in hardware. Real OTPs must never be test fixtures. The
implemented private-view transport is separate from the future continuous scrcpy stream;
Android mobile private-image viewing is not implied by the initial Android chat/feed client.
