# YoutubeBot private-data restore

Use this reviewed helper on the already-enrolled work PC after receiving every encrypted
archive part, the transfer manifest and snapshot index. It does not enroll the PC, change
its ZaturoBot keys/journal, execute incoming files, sign into Google or run video jobs.
The source YoutubeBot checkout must match the full reviewed revision in the protected
manifest. Existing work-PC files and browser sessions remain preserved.

Install the standard `age` and `zstd` command-line tools if missing. The native private
age identity is the one already installed at `~/.config/zaturobot/transfer.identity`;
never send it to GitHub or replace it to fix an archive error. Open the manifest/index
from their own UUID inbox folders, then run from the reviewed ZaturoBot checkout:

```bash
python3 deploy/services/restore_youtube.py \
  --manifest /absolute/inbox/MANIFEST-TRANSFER-UUID/youtube-transfer-manifest.json \
  --index /absolute/inbox/INDEX-TRANSFER-UUID/youtube-snapshot-index.json \
  --project /absolute/path/to/YoutubeBot \
  --apply-data
```

`--inbox` and `--identity` override their standard private local paths when necessary.
Without `--apply-data`, files are verified/decrypted/staged and existing project files are untouched.
There is no automatic execution of received instructions. Let any current YoutubeBot
batch/sign-in finish before applying a migration. The helper never stops another process.

Each part's size/SHA-256/SHA-512 is checked at its canonical UUID inbox path. The helper
concatenates encrypted parts directly into standard age decryption, authenticates the
full outer archive and verifies its encoded hash. It validates the decrypted zstd checksum
and every indexed tar-file size/SHA-256 before any project merge. The archive
creator's embedded `SNAPSHOT-INDEX.json` must match the authenticated external index
(exact bytes, with only the producer's single detached-file terminal newline optional); it is metadata and is not restored into the project. Extraction rejects
traversal, unapproved roots, duplicate entries, symlinks, hardlinks, devices, FIFOs and
sparse files. Bounds are 32 parts, 16 GiB encoded archive, 32 GiB unpacked data and
100,000 regular files. Plain data stays in private 0700 staging with 0600 files.

Only the approved `.local` workflow state and user media roots are merged. Missing files
publish atomically without replacing an existing path; identical files are skipped and
conflicts are reported. Existing browser profiles are quarantined as a whole, preventing
mixed Cookies/Preferences from different sessions. Old tracked/untracked source overlays
remain staged recovery data and never overwrite the improved Git source. File timestamps
are preserved, keeping the existing run-history ordering meaningful. Cross-filesystem
publication uses a verified temporary copy followed by a no-clobber link.

A private restore report records the staging location, restored/identical counts,
conflicts and quarantined paths. The original inbox parts and source data are not deleted.
Only the helper's temporary decrypted compressed container is removed after the attempt;
staged verified data remains for inspection. Same-filesystem publication shares inodes
with staging to avoid duplicating gigabytes; treat the staging as recovery workspace,
not an immutable second backup. The encrypted inbox parts remain the snapshot evidence.
Check free disk before restoring: beyond the received parts, staging needs the compressed
archive plus unpacked data and a safety margin. Owner edits are never silently replaced.

Browser credentials may be tied to the source OS keyring. Use the YoutubeBot account
window for interactive Google/ChatGPT sign-in on the target and exact channel verification.
Use the new public CLI; copied historical `.local/bot` scripts can contain old absolute
paths. The YoutubeBot runtime rebinds only its own extension/download preferences when
connecting an account. Do not globally replace strings in browser databases or history.

Hermetic verification (no actual accounts, media, network or Android device):

```bash
python3 -m unittest discover -s deploy/services -p 'test_*.py'
```

Tests cover malformed paths/archive types, checksums/missing entries, canonical part
locations, owner conflicts, existing browser profiles, quarantined source, read-only
repeat behavior and cross-filesystem publication. A separate local smoke exercised the
real standard age/zstd pipeline using generated synthetic identities and a tiny fixture.
