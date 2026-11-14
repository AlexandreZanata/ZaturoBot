# Android device security

The registered serial, capability list and an authenticated owner request are independent
checks. Device text, task prose, screenshots and model output cannot create a grant.
The current adapter supports USB registrations only; a TCP/IP observation cannot replace
one, and there is no raw ADB forwarding through the VPS. The owner completes Android RSA
consent physically. Revoke a lost notebook's debugging authorization on the phone.

## Local process and device boundary

The ADB executable is an absolute canonical regular executable without group/world write
permission. Calls use a minimal environment containing only the worker HOME, fixed
locale and system PATH, excluding model credentials and ambient ADB overrides. They pin
the loopback ADB host/port and always pass the registered `-s SERIAL` for a device operation. The only
current command shapes are device discovery, four fixed `getprop` queries and
`exec-out screencap -p`. There is no general shell input, text interpolation, network
pairing, package mutation, reboot, root, lock-screen PIN or installer operation.

Each client subprocess has its own process group, null stdin, bounded output and a
bounded deadline. Drop, timeout, output overflow and cancellation terminate its owned
process group through safe Rust APIs. Raw stdout/stderr is never put in an error or log.
The shared loopback ADB server is not an owned task process and is not killed to fix one
request. Status strings are bounded and reject control characters; screenshot buffers
use zeroizing memory and are capped before parsing dimensions. Image consumers must
perform their own bounded image decoding; the adapter checks PNG framing/dimensions,
not semantic image content or the absence of protected content.

`SessionManager` permits one active lease per configured device. Owner takeover revokes
its predecessor before granting replacement ownership. An expired/released/replaced
lease cannot capture pixels. In-flight operations observe revocation and erase/discard
their unreturned buffers. Agent/test actors cannot use sensitive-screen capture. Leases
are process-local and disappear on restart; restart requires new authorization, not
reconstruction from an old token. Session metadata contains no capability token.

## Sensitive pixels and protections

All phone captures are treated as potentially sensitive; the adapter cannot classify
an OTP-free screen reliably. `SensitiveFrame` deliberately has no `Debug` or `Serialize`.
Never add a convenience file dump, JSON/base64 result, model prompt, ordinary artifact
cache or normal VPS transfer upload for these bytes. Only the dedicated ephemeral owner viewer may consume them. Its session binds the
configured viewer device/age recipient, exact worker, registered phone and finite owner
grant. The worker atomically claims the grant once and verifies all fields before capture;
repeat claims, other workers and completed/consumed sessions are denied. A lost grant
response requires a new owner request rather than automatic duplicate capture. General feed history stores an operation category/result,
never screen text, image bytes, a code or a persistent sensitive screenshot thumbnail.
Hub sessions contain age ciphertext in bounded RAM only, with active one-second expiry
cleanup and no-store HTTP responses. The desktop decrypts and bounds PNG decoding in
memory, clears its image on dismissal/navigation/focus loss and respects the server's
absolute image expiry. This describes application retention; no claim of forensic OS/GPU
memory erasure or prevention of an owner's external screen recorder is made.

Do not bypass Android lock screens, secure windows or redacted notifications. A black
or unavailable screenshot does not grant permission to change Android settings.
[FLAG_SECURE](https://developer.android.com/reference/android/view/WindowManager.LayoutParams#FLAG_SECURE)
can prevent protected content from appearing in captures. The bot must report the
limitation and require the owner's physical action where necessary.

Dedicated worker OS isolation and reviewed USB permissions remain deployment work.
Protect the default `~/.android` keys under the worker's private user scope (ambient
`ADB_VENDOR_KEYS` is intentionally not inherited); neither the hub nor another device receives those keys. Do not run the worker as
root or grant broad USB access as a generic reconnect remedy.
