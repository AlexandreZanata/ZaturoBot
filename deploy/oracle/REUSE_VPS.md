# Repurposing the existing Oracle host

Status on 2026-11-13: preparation authorized; remote inventory and cleanup blocked by
missing working SSH access. No remote files, services, keys, firewall, or certificates
have been changed by this implementation pass.

The owner explicitly requires preserving every local file under
`/home/iiii/PESSOAL-PROJETOS-ALEXANDRE/professional-pdf-generator/`, including its
certificates and configuration. That repository was inspected read-only. Cleanup applies
to the identified Oracle VPS, never to the local project or unrelated hosts.

## Access preparation

The ignored root `.env` has mode 0600 and contains separate generated local service
tokens plus deployment connection variables. The private-key path remains unset until
a working existing identity is found or administrative access is recovered. No private
SSH material is embedded in `.env` or tracked in this repository. Deployment variables
are workstation settings and are not consumed by either Rust runtime.

The former application's runbook identifies an Ubuntu administrative account and an
Oracle IPv4 address. The existing saved SSH host identity was retained. Two currently
loaded local ED25519 identities failed public-key authentication on port 22; the
documented alternative port 22422 timed out. No host-key verification was disabled.

File/path and private-key-header searches covered the home directory and `/data`, which
backs Downloads and SSD links. A separate metadata-only search inspected entry names in
30 ZIP/tar backups. No Oracle-specific working key was identified. Known signing/test
keys and unrelated Hostinger keys were not repurposed. Searches did not change originals
or reveal private-key contents. This does not establish that an offline or encrypted
backup cannot contain the original key.

Once access is available, set the actual host/user/port/key path in `.env` and run:

```bash
set -a
source .env
set +a
bash deploy/oracle/inspect-vps.sh
```

The script is read-only and requires a previously verified saved host key. A replacement
instance or host key requires independent fingerprint verification before updating trust.

## Cleanup boundary

First capture the live service, timer, listener, firewall, and storage inventory. Identify
the previous application's exact unit files, releases, data, backup, credential, and
certificate paths; a stale runbook alone is insufficient evidence for deleting a path.
Record whether any unrelated application or valuable remote-only data remains. Resolve
unclear data ownership before deletion. Keep a private recoverable backup when required.

For confirmed obsolete PDF deployment resources, stop/disable their services and timers
before removing verified application-specific files. Preserve working SSH access, host
keys, administrative users, OS essentials, security updates, and baseline firewall
protection. Do not run an unrestricted filesystem wipe, recursive home deletion, blanket
package purge, or commands copied from the old deployment that resynchronize/delete
local content. No destructive cleanup script is generated before live inventory.

Then create the dedicated hub user/state/config paths and apply the reviewed native
service profile only within the owner's preparation/deployment authorization. Keep the
hub on a private path and do not open a public control port. Installing or starting the
hub, choosing VPN/TLS access, and recovering SSH authorization require concrete operator
decisions. The current pass has not performed those actions.
