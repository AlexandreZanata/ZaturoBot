# Installed OpenCode execution

Own the fixed installed OpenCode runner, job-scoped MCP broker, one-attempt Go gateway and
metadata-only execution receipts. Core owns typed work orders. Worker composition supplies
the existing durable budget port; all SQL remains in zaturo-db. YoutubeBot remains the media
implementation through zaturo-youtube's fixed argv adapter.

Default-deny actual V2 tool/provider policy is mandatory. Never expose shell, filesystem,
arbitrary ADB, approval, generic HTTP or bot_chat tools. Never inherit worker/admin/provider
environment credentials. Reuse the installed OpenCode's protected integration; its key may
cross only the loopback inference gateway to the fixed Go endpoint, without logging.

Persist intent before launch and reconcile the same operation after uncertainty. One paid
forward is the limit, even if OpenCode retries internally. Settle actual usage and verify Go
billing/cost before releasing a tool call. A broker dispatch receipt is not media completion.
Stopping the dispatch process is distinct from cancelling its detached media actor.
Synthetic tests use local transports/fake processes only; real acceptance is owner-authorized.
