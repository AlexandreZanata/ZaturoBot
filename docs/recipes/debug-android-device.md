# Debug the registered USB phone

Read [integration](../ANDROID_INTEGRATION.md) and use the worker's Android discovery/status
command. It reports bounded metadata; do not dump screen data, notification contents,
private ADB keys or an entire environment for troubleshooting.

- No listed serial: verify a USB data cable and Developer Options/USB debugging physically.
- `UNAUTHORIZED`: unlock the phone and accept this notebook's RSA consent dialog. The bot
  cannot replace that physical confirmation.
- `CONNECTING`/offline: reconnect the intended phone and inspect its exact serial. Do not
  switch to another ready phone or kill the shared ADB server as an automatic repair.
- USB permission error: review the distribution's specific udev/user-group setup; avoid
  running the worker as root or granting access to every USB device.
- Registered phone missing while others appear: keep the selected registration and report
  the mismatch. Never identify by a USB-port position alone.
- Timeout: inspect cable/phone responsiveness and the configured trusted executable;
  retain bounded deadlines rather than replacing them with infinite waits.
- Protected/black screen or post-reboot lock: report unavailable protected content and ask
  for the required physical owner action. Do not disable secure-window protections.

A separately installed `run cell doctor` from the read-only CelularLinuxADB reference can
help the owner with local setup. It is not the bot's permission system and its free-form
options are not accepted by the typed adapter. For remote sensitive viewing, first check
that the ephemeral owner-view activation gate is actually implemented; receiving a local
PNG or seeing a phone in discovery does not satisfy that gate.
