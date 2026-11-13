# Oracle host reuse: verified operational report

Historical checkpoint: owner-authorized cleanup was independently verified through a
new SSH session on 2026-11-13 at 18:48 America/Cuiaba (22:48 UTC). The retired PDF
deployment was backed up, then its confirmed resources were removed. At that checkpoint
the host was prepared for a future hub; no hub binary/service had been installed and no
Git push had occurred. Subsequent owner-authorized activation is separate from this
cleanup evidence and the original local foundation benchmark. See
[the live deployment runbook](../../docs/LIVE_DEPLOYMENT.md) for current service activation.

## Scope and local preservation

The owner explicitly authorized cleanup of the identified Oracle VPS and required
preserving every local `professional-pdf-generator` file, including certificates,
keys, and configuration. Cleanup commands targeted the remote host only. A before/after
local inventory matched all 3,634 entries by size, modification time, and mode, and
all eight recorded certificate/key/environment SHA-256 hashes remained unchanged.
The original supplied SSH key files were not modified. This evidence documents the
preservation checks performed; it is not a claim to have hashed every local project file.

OS packages, Oracle/cloud agents, administrative access, system users, SSH host keys,
baseline journals, and baseline sysctl settings were retained. There was no blanket
filesystem wipe, recursive home deletion, package purge, or local synchronization cleanup.

## Access and private recovery material

The owner supplied the existing RSA SSH identity and explicitly selected port 22.
Authentication succeeded as the existing Ubuntu administrative account with saved-host
verification retained. No host-key checking was disabled. The confirmed host address
and credential values are intentionally absent from tracked documentation.

At the owner's request, the key pair was copied into ignored `.secrets/ssh/` with
mode `0700` directories and `0600` files; originals under `/data/downloads` remained
untouched. The ignored root `.env` has mode `0600`, references the private key's absolute
path, and sets SSH port 22. These SSH variables are deployment-workstation metadata,
not runtime settings consumed by either Rust application. Existing generated local
service tokens were not deployed to the VPS.

Before any remote removal, the retired application's data, releases, configuration,
credentials, certificates, and old backups were captured in the encrypted local archive:

```text
.secrets/vps-backups/oracle-pdf-2026-11-13/retired-pdf.tar.gz.age
```

The archive is 774,349,505 encrypted bytes. Full authenticated age decryption, gzip,
and tar traversal validated 8,471 members, 23 expected roots, and four critical paths.
It includes the retired system's four documents and two signature records along with
its other private deployment material. Private verification records retain detailed
inventory/hash evidence rather than exposing it in committed documentation.

The matching recovery identity is `.secrets/ssh/ssh-key-2026-05-28.key`; keep it and the
encrypted archive private and recoverable. Encryption does not make it safe to commit
the private key. Both `.env` and `.secrets/` passed ignore checks, and Git's tracked-file
query returned no entries for them. No private key content, real token, host address,
old credential, or certificate was added to tracked examples.

A future restore must authenticate/decrypt the archive and extract into a private
staging directory for review before selectively restoring anything. Never extract it
blindly over the local PDF project, current hub preparation, or live root filesystem.
No restore was performed during preparation.

## Exact remote cleanup boundary and verification

The confirmed obsolete application paths were:

- `/opt/ppg`
- `/var/lib/ppg`
- `/var/backups/ppg`
- `/etc/ppg`
- `/var/log/ppg`
- `/home/ubuntu/ppg-src`

The application and its named timers/services were stopped and disabled. Removal was
limited to those confirmed directories, seven application unit files, the specifically
identified application credential-store files, old application fail2ban jail/filter,
and retired Caddy configuration/state/log directories. Caddy was disabled. Packages
and unrelated OS services were retained.

SSH protection was preserved by installing `/etc/fail2ban/jail.d/zaturo-sshd.conf`
with the existing `[sshd]` protection, validating fail2ban, and reloading it before
removing the obsolete application-specific jail/filter. Firewall changes removed only
obsolete HTTP/HTTPS allowances; incoming policy remains deny, with only `22/tcp`
allowed for IPv4 and IPv6. This does not authorize opening a public hub port.

The first cleanup attempt stopped before any removal when a firewall-output-format
precheck did not match. The check was corrected to use verbose UFW status, then rerun;
its SSH-preservation assertion was retained. Independent post-cleanup verification
confirmed all 18 checked obsolete deployment/data/unit paths absent, Caddy disabled,
SSH and fail2ban active, and only the `sshd` fail2ban jail present. No listeners remained
on ports 80, 443, 8000, or 7878. Baseline OS RPC port 111 remained listening and was
blocked by the deny-incoming firewall; it was deliberately outside application cleanup.

## Prepared hub identity and filesystem

Preparation created a dedicated non-login `zaturo-hub` identity and the following
reviewed access boundaries:

- `/etc/zaturo`: `root:zaturo-hub`, mode `0750`.
- `/etc/zaturo/hub.toml`: `root:zaturo-hub`, mode `0640`.
- `/var/lib/zaturo-hub`: owned by `zaturo-hub`, mode `0700`.
- `/opt/zaturobot`: root-owned, mode `0755`.
- `/etc/zaturo/zaturo-hub.service.staged`: root-owned, mode `0600`.

The hub user successfully read and parsed the TOML configuration. Its bind is private
loopback `127.0.0.1:7878`, and its database path is absolute beneath hub state. Group-read
access to the nonsecret configuration is required; a root-only config directory/file
would prevent this service user from starting. Any future service credential environment
file remains separately root-owned mode `0600`, read by the system systemd manager.

The unit is staged outside the installed unit location and is not enabled. No binary,
service credentials, provider configuration, VPN, TLS termination, or ZaturoBot service
was installed or started. Read the [installation profile](README.md) and obtain the
separate private-network/service activation decisions before deployment.

## Host snapshots and remaining work

The post-cleanup snapshot reported 952 MiB total RAM, 256 MiB used, and 696 MiB available;
the earlier snapshot reported 368 MiB used and 583 MiB available. `df -h` reported
4.5G used on a 48G filesystem after cleanup versus 5.3G used before. These are host
snapshots, not ZaturoBot memory benchmarks, and do not establish application capacity.
The hub had not run on this host at the end of the preparation checkpoint.

At that checkpoint, remaining deployment work was to choose the private access path,
review target binary compatibility and service credentials, install an authorized
release, activate the reviewed unit, and measure the real target workload. The owner
subsequently authorized an SSH-tunneled installation; its current activation evidence
is separate from these historical cleanup observations. The local
[foundation benchmark](../../docs/benchmark-local.json) is independent evidence from a
development machine. Ongoing backup/recovery, firewall, logging retention, updates,
and SSH-key protection remain operator responsibilities.

For a later read-only inventory using the already verified host identity:

```bash
set -a
source .env
set +a
bash deploy/oracle/inspect-vps.sh
```

The script performs inspection only. A replacement host or changed host key requires
independent fingerprint verification before changing saved trust.
