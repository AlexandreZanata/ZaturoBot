# Add a typed Android operation

1. Check [Android integration](../ANDROID_INTEGRATION.md), the existing adapter and
   [device security](../ANDROID_DEVICE_SECURITY.md). Reuse a fixed supported operation.
2. Define pure strict parameters/results in core, with an explicit registered device ID,
   capability, timeout/output cap and category-only failures. Never accept raw shell or
   task-supplied argument arrays. Distinguish owner, agent and test actors.
3. Put exact ADB/scrcpy invocation and owned process cleanup in `zaturo-android`; preserve
   serial binding, physical authorization checks and cancellation/revocation fencing.
4. Register the new callable capability/tool in canonical inventory and application
   policy. Add orchestration and persistence at their existing boundaries; a contract
   or library method alone is not a remotely enabled tool.
5. Add fake-executable fixtures for normal output, denial, malformed output, timeout,
   overflow, disconnect, identity mismatch, cancellation and an uncertain side effect.
   Never test against a real inbox or silently opt into attached hardware.
6. For pixels, use [sensitive-screen requirements](../ANDROID_SENSITIVE_SCREENS.md).
   Do not serialize a `SensitiveFrame`, store it as an ordinary artifact or add a model
   adapter. Prove ephemeral owner delivery before remote activation.
7. Update current/planned status and ADRs, run the focused tests and `cargo xtask check`,
   and report actual hardware evidence separately from synthetic fixture success.
