# Native Android companion

Kotlin Android client only. Network requests are bounded and use a scoped app token.
Never add READ_SMS, notification harvesting or accessibility permissions to make remote
phone automation work: ADB operations belong to the Linux worker and explicit scopes.

`SshTunnel.kt` owns lifecycle-scoped pinned SSH local forwarding, never shells/SFTP or
remote forwarding. `HostPin.kt` hashes the SSH wire key with constant-time comparison.
`PrivateSettings.kt` stores enrollment/token as Android-Keystore AES-GCM ciphertext;
never log an enrollment or persist a decrypted SSH key. Backups are disabled. Poll only
while visible. Normal hub feed rejects sensitive images. No plaintext SMS/code cache.
Tests use generated keys/synthetic origins; real-device testing must be explicit.
