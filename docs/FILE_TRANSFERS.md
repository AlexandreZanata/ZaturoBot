# Encrypted device files

Implemented in 0.3.0 as an owner-authorized Phase 2 extension. Two Linux computers can
send files through the private VPS even behind different routers. Both initiate outbound
SSH connections on port 22; no PC listener, inbound router rule or public hub port is
needed. The hub remains at 127.0.0.1:7878, forwarded to 127.0.0.1:17878 on each PC.
Actual throughput depends on both Internet connections, disk, encryption CPU and VPS
bandwidth. This is store-and-forward: uploading and downloading are separate operations.

## Authentication and scope

A private `devices_file` contains 1–8 unique device IDs, SHA-256 bearer digests and native
age X25519 public recipients. The hub never receives a decryption identity. When this
registry is configured, the legacy shared credential is disabled. A credential binds to
one device ID before the WebSocket hello; a token cannot register as a different PC.
Registry changes take effect on a hub restart, disconnecting revoked sessions. Remove
the corresponding SSH authorized key as well to revoke tunnel access.

The owner credential retains task creation/control for any registered worker. A device
credential can inspect its own task results and submit only the existing read-only
diagnostic to itself. It cannot use owner endpoints, request paid AI or execute arbitrary
commands. The work-PC profile has no owner credential or provider key. Existing home
public-coding DeepSeek policy is unchanged. Remote desktop, browser research and social
account actions require separate future implementations and authorization.

The `zaturo-link` SSH user accepts only public keys with restricted local TCP forwarding
to 127.0.0.1:7878. Shell, command execution, SFTP, PTY, agent/X11 forwarding, Unix socket
forwarding and remote forwarding are disabled. Separate endpoint keys can be revoked.
Admin access remains separate. Host verification uses a separately pinned known-host key;
neither enrollment nor the services disable strict checking.

## Sending and receiving

The Linux installer creates `~/.local/bin/zaturo`, loading only the private worker env.
Examples below use that wrapper; it can also be invoked by absolute path.

```bash
zaturo files peers
zaturo files send /absolute/path/report.pdf --to work-pc
zaturo files resume UUID-PRINTED-AS-RESUME-KEY
zaturo files list --outbox
zaturo files status TRANSFER-UUID
zaturo files list
zaturo files sync
zaturo files receive TRANSFER-UUID
zaturo task diagnostic
zaturo task show TASK-UUID
```

Send encrypts locally to the recipient and sender public keys using standard age streaming
X25519 encryption. A bounded encrypted header contains the plaintext name, byte count
and SHA-256. The encrypted upload and idempotent request are persisted in the private
cache before admission. The printed resume key identifies that request, not necessarily
the hub transfer UUID. Repeating resume reconciles the hub checkpoint and sends only
remaining ciphertext; an uncertain network response never blindly appends a second copy.
Network errors and transient 408/429/502/503/504 replies receive at most four idempotent
attempts with bounded backoff; other failures require explicit resume/review.
Successful uploads remove their large local ciphertext; the small admission record remains.
An encryption failure removes its temporary ciphertext. A crash during preparation before
the request is written can leave an unreferenced private `.send.age` file; inspect before
removing it and never delete an active cache to fix connectivity.

Workers check their inbox every five seconds, independently of WebSocket heartbeats and
model execution. Ready files wait for an offline destination until expiry. Downloads
resume from a private ciphertext partial; final ciphertext SHA-256, age authentication,
inner plaintext byte count and SHA-256 must all pass before publication. Each transfer
uses `~/.local/share/zaturobot/inbox/TRANSFER-UUID/original-name`; identical basenames from
different transfers cannot collide. Hard-link publication never overwrites an existing
file. Delivery acknowledgement follows verified publication. Repeating an identical
receipt is safe; a locally edited destination is preserved and requires explicit review.

Files are data. Receipt never opens, executes or sends them to a model. The hub can see
device IDs, filename, encrypted length/hash, timestamps and delivery state; this is not
metadata anonymity. Losing both endpoint identities makes retained ciphertext
unrecoverable. Back up identities privately. The VPS transport account cannot decrypt.

## Bounds and recovery

Defaults are 2 GiB per encrypted file (plaintext must fit with age/header overhead), 8 GiB
reserved spool, 128 nonexpired transfers and 24-hour retention from admission. Hard maxima
are 4 GiB/file, 16 GiB/spool, 512 transfers and seven days. Default and maximum chunks are 4 MiB, avoiding a round trip per small block. HTTP admission allows only two simultaneous device requests,
120 seconds per file operation and 32 total TCP sockets. Registry-enabled hub profiles
require `max_page_size >= 20`; the inbox snapshots at most 512 pending metadata records
across bounded pages so a page of invalid files cannot starve later valid arrivals. Streaming encryption/hashing
and bounded chunks avoid loading whole files into RAM. Sender and recipient each need
local disk for ciphertext plus the original/received plaintext. The inbox is owner-retained;
no automatic local plaintext deletion is performed. Remove unwanted completed inbox
folders and obsolete local cache metadata explicitly to manage disk usage.

SQLite reserves the complete encrypted size before upload. Delivered transfers still
consume quota until their actual bytes are removed. A serialized disk/checkpoint critical
section fsyncs bytes before recording progress. Extra uncommitted bytes left by a crash
are truncated to the durable offset before the next append. Exact acknowledged chunk
replay verifies the existing bytes. Gaps, overlaps, conflicting retries, traversal names,
noncanonical UUID paths, FIFOs and symlink file destinations are rejected. New spool
objects, directory entries, publication and expiry removals are synced before durable
acknowledgement or quota release. Local advisory locks
prevent the worker and CLI from receiving/resuming the same transfer concurrently.
A CLI receipt racing the automatic inbox can report an active-transfer conflict; retry
or inspect the inbox after the worker finishes.

Every minute, a bounded cleanup batch removes expired ciphertext before recording
`expired` and releasing its quota. Task history and transfer audit metadata remain. An
expired interrupted upload needs a new send; its old idempotency key cannot resurrect it.
A corrupt download remains private and unacknowledged. Failed items do not prevent
valid files in later pending pages from being received. Stop active receipts, inspect/remove
only that UUID's ciphertext partial, then explicitly retry. Authentication key replacement
requires stopping the device and coordinating its registry entry; do not reuse a different
PC's worker database or encryption identity.

Hub schema 1 upgrades transactionally to 4; worker journal schema stays 3 and task
WebSocket protocol stays 2. File endpoints are additive authenticated HTTP APIs:
`/v1/device/{me,peers,limits,tasks}` and `/v1/device/transfers`, plus UUID-specific status,
`chunk?offset=N&length=N` (GET/PUT), `complete` and `ack`. Listing uses `incoming`, `pending`,
`limit`, `offset`; pending selects the recipient's ready inbox. Only sender/recipient can
inspect a transfer; only the sender can upload/complete and only the recipient can ack.
All transport is inside the verified encrypted tunnel or a separately configured HTTPS
origin. Never expose loopback HTTP directly on a public interface.

## Enrollment and validation

See [Linux device installation](../deploy/device/README.md). The public repository has
only an installer and templates. A private owner-issued JSON bundle carries a new device
SSH key, device bearer, native age identity and pinned SSH host key. Its containing prompt
and all credentials stay under ignored `.secrets/` and must never enter GitHub, issues,
logs or screenshots. It contains neither the VPS admin key nor the AI provider credential.

Tests use generated age identities, synthetic tokens, real loopback HTTP/WebSocket and
separate SQLite/cache roots. They cover interruption/orphan recovery, exact replay,
wrong identity, ciphertext tampering, no overwrite, traversal/symlinks, spool admission,
retention/quota release, schema upgrade, bidirectional delivery and device diagnostic.
Live VPS results are recorded in [the transfer report](FILE_TRANSFER_REPORT.md). The
physical work PC must run the handoff checks on its actual network before claiming
cross-network acceptance; a second profile on the home PC only simulates that device.
