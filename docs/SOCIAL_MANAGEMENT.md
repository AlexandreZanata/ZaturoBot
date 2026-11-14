# Social-management foundation

The optional social adapter connects bounded worker tasks to a separately operated
[Postiz Public API](https://docs.postiz.com/public-api/introduction). Postiz supplies
network integrations and its own publishing scheduler; ZaturoBot supplies exact owner
review, local grants, durable intent fences and status reporting. It is a foundation for
supported account actions, not universal social-network management.

Postiz's [application source](https://github.com/gitroomhq/postiz-app) is AGPL-3.0;
this adapter implements its documented API boundary without copying/importing that
application. Run any chosen Postiz deployment separately on a suitable worker/service
host. Do not put Postiz/Docker, its accounts, or its database on the lightweight hub VPS.
Existing YoutubeBot extension, generation, profiles, upload and scheduling remain in
their independent implementation and are not migrated to Postiz.

## Implemented and unavailable actions

The fixed [Python adapter](../deploy/computer/social_service.py) exposes capabilities,
readiness, prepare, execute and status. Initial post platforms are `threads`, `mastodon`,
`bluesky`, `telegram`, `nostr` and `vk`: their documented payload settings require only
`__type`. Other platform schemas need their own reviewed adapter rather than guessed
settings. YouTube requests use the existing [YoutubeBot service](YOUTUBE_SERVICE.md).

Actions are immediate publication, scheduling and saving a draft in Postiz. Media must
already be uploaded and explicitly registered locally; there is no arbitrary URL/file
upload tool. Comment reading/replies, direct messages, deletion, follow/unfollow and
account login are unsupported in this initial connector. The documented Public API
examined here does not establish a general social comment-reply route. Postiz's internal
collaboration comments are not interchangeable with network comments. Do not simulate
those actions or fall back to an unrestricted browser/shell path.

The implementation uses the official [integration list](https://docs.postiz.com/public-api/integrations/list),
[API-key connection check](https://docs.postiz.com/public-api/integrations/is-connected),
[create-post](https://docs.postiz.com/public-api/posts/create) and
[list-posts](https://docs.postiz.com/public-api/posts/list) contracts, checked 2026-11-14.
Self-hosted routing must actually expose the configured `/api/public/v1` or `/public/v1`
base; a redirect to an internal dashboard is not a compatible API.

## Exact draft, approval and execution

`prepare` is local and inert. Its request identifies one durable operation UUID, one
allowlisted integration, `publish|schedule|draft`, bounded content and optional approved
media IDs. A schedule supplies an explicit UTC `YYYY-MM-DDTHH:MM:SSZ` time, 60 seconds to
365 days ahead. It does not infer an ambiguous local timezone. Content is at most 1500
UTF-8 bytes; the planner uses a smaller 1200-byte target for readable native confirmation.
No network publication occurs during preparation.

Preparation persists the exact canonical Postiz payload and returns its bound SHA-256,
operation ID, expiry and readable `preview_pt`. The hash also binds the destination,
current private configuration/credential fingerprint and operation identity. The native
owner confirmation must show the exact content/account/action/time and match the earlier
same-parent prepare result. Model-generated descriptions cannot replace that evidence;
unreadably large previews are rejected rather than silently truncated for approval.

The coordinator separately requires `/aprovar UUID` through the native composer's
`computer_confirm` action for the pending execution step. The same prefix in ordinary
assistant/MCP chat remains inert; MCP tools cannot select native confirmation transport.
The model cannot provide `owner_approved`. The Rust adapter derives that flag from the
current authenticated grant. The helper additionally requires local `allow_execute=true`,
matching operation/hash, unchanged config, an unexpired draft and no uncertain/in-flight
operation on that integration. Local revocation/config and elapsed expiry are rechecked
after the read-only preflight, immediately before durable intent. An exact already
accepted operation returns its cached receipt; it never becomes another POST.

Readiness calls `GET /is-connected` and `GET /integrations`. A valid API key and
`disabled=false` establish that the configured account is registered, not that the social
network's current session/consent is usable. Disabled/missing/unknown accounts are
reported distinctly. The owner signs in/connects the correct account in Postiz and the
network's actual consent UI; no credential copying or challenge bypass is performed.
Raw provider errors, tokens and unrelated account records are not returned to chat.

After the write fence is durably saved, execution makes one `POST /posts` with the exact
prepared payload. No redirect, HTTP proxy, automatic retry, alternative account or
alternative route is used. A lost/malformed response, interruption or failure to persist
the response leaves an intent/uncertain fence. Reopening/reconnecting or choosing a new
operation ID cannot silently publish again while that account's outcome is unresolved.
Inspect the Postiz dashboard and reconcile evidence before further publication; do not
delete the record or edit it merely to hide uncertainty.

## Acceptance is distinct from publication

Postiz create-post returns an accepted post ID. That is neither a completed network
upload nor proof of publication. `status` uses a bounded `GET /posts` date range and
matches the stored post ID plus exact integration. Unrelated posts/content matches cannot
prove an uncertain operation's outcome.

Observed `PUBLISHED` is reported as Postiz's publication evidence; `QUEUE` means scheduled,
`DRAFT` means saved draft and `ERROR` means failure. Missing/unrecognized records remain
unconfirmed. Reports attribute this evidence to the API, not to a screenshot. Native
normalization retains acceptance/scheduling as partial work and leaves unknown outcomes
uncertain. It must not turn HTTP200 into “video uploaded successfully.”

If the owner requests a screenshot, the independent computer private-view path captures
only the currently granted window. This adapter itself produces no screenshots. A frame
is ephemeral, endpoint-encrypted and excluded from models/history; it is not an automatic
proof of a Postiz API action. See [computer use](COMPUTER_USE.md).

Stopping the computer bridge or cancelling a ZaturoBot task stops future dispatch; it
cannot undo a click, immediate post, or schedule already accepted by Postiz. A schedule
runs under Postiz's independent service after submission. Removing/revoking an accepted
schedule needs the owner's deliberate action in Postiz; this connector has no delete or
automatic rollback operation.

## Private local configuration and CLI

Use Python 3.11+ and a private TOML profile, separate Postiz API-key file and JSON receipt
journal. Profiles/keys/receipts are mode0600, their private directory mode0700. Symlinks,
foreign/corrupt receipt files, unsafe origins and broad credential permissions fail
closed. The key never belongs on the hub, in the APK, model context, arguments or Git.
The Postiz header is `Authorization: API_KEY`, not a Bearer header.

A minimal profile has these fields; the addresses, IDs and paths below are placeholders:

```toml
schema = 1
enabled = false
api_base = "https://postiz.example.com/api/public/v1"
allowed_origin = "https://postiz.example.com"
api_key_file = "/absolute/private/postiz.env"
journal_file = "/absolute/private/social-receipts.json"
allow_execute = false

[allowed_integrations]
actual_integration_id = "bluesky"
```

The key file contains only the raw API key or one `POSTIZ_API_KEY=...` assignment. Inspect
an existing instance and configure actual integration IDs before enabling read access.
`allowed_origin` must exactly match the API origin: HTTPS, or literal-loopback HTTP for
a local service/private tunnel. Credentials, query/fragment URLs, redirects and ambient
HTTP proxies are not accepted. At most eight account IDs are locally registered.

Optional `[allowed_media]` maps at most 32 already uploaded Postiz media IDs to exact
same-origin media URLs. Requests select up to four of those IDs; they cannot provide a
new URL/path or use upload-from-URL. No media is downloaded/executed by this helper.
Enabling a config or preparing a draft still grants no publication by itself.

The fixed interfaces accept one bounded JSON object on stdin and return one bounded JSON
result; no provider body or stack trace is part of the protocol:

```bash
python3 deploy/computer/social_service.py capabilities </dev/null
printf '{}\n' | python3 deploy/computer/social_service.py readiness --config /absolute/private/social.toml
python3 deploy/computer/social_service.py prepare --config /absolute/private/social.toml < /absolute/private/draft-request.json
python3 deploy/computer/social_service.py execute --config /absolute/private/social.toml < /absolute/private/approved-execution.json
python3 deploy/computer/social_service.py status --config /absolute/private/social.toml < /absolute/private/status-request.json
```

The worker supplies typed requests and current owner-grant evidence; these are not
instructions to run a real publication for a setup test. `execute` accepts only the exact
prepared `operation_id`, `approved_draft_hash` and current owner-approval flag. `status`
accepts that operation ID; it never resubmits a missing post.

The local provider journal uses flock and bounded atomic JSON receipts, fsyncing the
file before replacement and the directory afterwards. It caps 500 operations/8 MiB, keeps
clock/expiry and uncertain fences, and never auto-removes old operations to make room.
It contains drafts/receipts, not the API key or screenshot bytes. Coordinator approval SQL
remains in `zaturo-db`; the helper owns only its transport-side receipt files.

## Evidence and remaining setup

Synthetic tests exercise the actual supported HTTP paths against localhost, raw-key
header handling, disabled proxies/redirects, exact approvals, configuration changes,
expiry, concurrency, crash recovery and response-persistence failure. File/dir fsync
ordering, corruption preservation and capacity rejection are also verified. They use
synthetic credentials/accounts only. No Postiz deployment, real consent, publication,
comment, media upload or actual work-PC screenshot is implied by these tests.

The destination still needs an actual compatible Postiz service, configured accounts and
local permissions. Model planning now uses explicit Personal/Go under ADR-0021, with
subscription billing proof and provider Extra Usage off. Historical Console402 is not a
current Go prerequisite, and no new live Go/account success is implied. Do not fall back
to Console credits or another model; a receipt check cannot reverse a provider charge.
Direct existing
YoutubeBot commands retain their separate typed/local grants and actual job observers.
