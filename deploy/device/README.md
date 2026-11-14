# Ubuntu/Debian device enrollment

This installer is for files and registered diagnostic tasks, without desktop control or
an AI key. It installs user services, a pinned outbound SSH tunnel and an encrypted inbox.
Use only an owner-issued protected enrollment bundle. Do not copy the VPS admin SSH key,
owner bearer, another device's database or provider credential.

On the destination, clone the repository, check out the exact `source_commit` in the
private bundle, install the pinned Rust toolchain and Linux build prerequisites
(`build-essential`, `pkg-config`, `git`, `curl`, `openssh-client`, Python 3, user systemd).
A working C compiler is needed for bundled SQLite/crypto; builds use Rustls rather than
requiring an OpenSSL runtime. Review the code before installing any missing OS packages.
Build on the PC, not the 1 GB VPS:

```bash
cargo build --release -p zaturo-worker --locked
chmod 600 /private/path/work-pc.enrollment.json
python3 deploy/device/install.py --bundle /private/path/work-pc.enrollment.json \
  --binary target/release/zaturo-worker
```

Do not interpolate bundle credentials into command arguments or print the bundle. The
installer refuses symlinks, unsafe permissions, an unexpected source commit, a wrong
binary version, or a conflicting existing profile. `--no-start` installs files for review.
`--home /absolute/private/test-home --no-start` supports isolated installer verification.
Keep existing profiles/journals; do not wipe a previously configured PC to bypass a conflict.

It installs private configuration/SSH/age material in `~/.config/zaturobot/`, executable
in `~/.local/lib/zaturobot/`, state in `~/.local/share/zaturobot/`, user units in
`~/.config/systemd/user/` and wrapper `~/.local/bin/zaturo`. It enables both services and
attempts user lingering for operation after logout. If local policy denies lingering,
finish that specific OS step through the machine administrator and verify it; active
services alone do not prove logout/reboot persistence. Do not run the worker as root.

Verify without printing credentials:

```bash
systemctl --user is-active zaturo-hub-tunnel zaturo-worker
loginctl show-user "$USER" -p Linger
~/.local/bin/zaturo files peers
~/.local/bin/zaturo task diagnostic
~/.local/bin/zaturo task show TASK-UUID
~/.local/bin/zaturo files send /absolute/path/a-nonsecret-test-file --to local-worker
~/.local/bin/zaturo files list
```

Confirm the diagnostic reaches `COMPLETED` and the home PC receives the exact test hash.
Send a second file from home to `work-pc`; verify its hash in the work inbox and `delivered`
at the sender. Restart both user services and verify the journal, inbox and connection
survive. Report the actual Linux distribution, architecture, source revision, service
states and end-to-end results. If the network blocks outbound port 22, report that connectivity failure and preserve
strict host verification and the private hub binding.

Revocation: remove this device's registry entry on the VPS, remove its authorized SSH key,
restart the hub and terminate this user's active device SSH session; preserve audit
records. Other devices have independent keys. Exact deployment evidence belongs in
[the report](../../docs/FILE_TRANSFER_REPORT.md).
