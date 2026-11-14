# Android USB adapter

Own fixed serial-bound ADB subprocesses, output parsing, bounded process lifecycle,
revocable device leases and memory-only sensitive-screen primitives. Pure contracts
belong to `zaturo-core` (`src/android.rs`); orchestration/persistence and endpoint authorization
remain outside this crate. Never run a task-provided command or arbitrary `adb shell`.

Discovery is read-only. Every capture/watch requires an owner-created, unexpired,
serial-bound sensitive-view lease. Screenshot bytes must never implement Debug/Serialize,
enter an LLM, ordinary task/feed history, logs or the normal VPS file spool. This crate
returns bytes only to the trusted worker private-view helper, which verifies the
one-time hub grant and encrypts to the registered desktop recipient. Keep remote
activation disabled by default until local USB/sensitive grants and the physical phone
are explicitly prepared. A changed image
is not evidence that an SMS or OTP arrived. Do not bypass USB RSA consent, the lock screen,
FLAG_SECURE or Android notification redaction. CI uses fake ADB executables and synthetic
PNG data only. Spawn failures retain only the safe OS error kind/code; never attach the
raw error string, executable path, serial or captured output. Executable-writing test
fixtures share an async lifecycle mutex because concurrent forks can inherit a writer
FD before CLOEXEC and cause Linux ETXTBSY. Keep within-test takeover/cancellation
concurrency and deadlines intact; do not add production retries to hide fixture races.
See [Android integration](../../docs/ANDROID_INTEGRATION.md) and
[sensitive screens](../../docs/ANDROID_SENSITIVE_SCREENS.md).
