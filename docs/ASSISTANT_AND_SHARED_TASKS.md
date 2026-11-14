# Native assistant and shared coordination

The owner-authorized extension reuses authenticated durable tasks and both native clients.
It does not introduce an unrestricted executor, automatic Codex runner, SMS reader or
publication grant. The implementation contracts live in core/activity.rs, core/service.rs,
core/intent.rs and core/shared.rs. This document explains those contracts.

## Chat behavior and token bounds

The updated Android/Linux chat explicitly submits `action=assistant`. Older API clients
that omit the action continue saving inert ordinary messages. Known explicit commands
skip model inference: `diagnóstico`, `canais`, `celulares`, `abrir sms`, and generation
requests such as `faça 4 videos do fallem price`. Only the owner's bounded command is
parsed; negations, conditions, unknown channels and extra operations do not become tools.
The alias resolves to the existing `fallem-prince` channel, count 1–10. Generation does
not authorize upload or scheduled publication.

Other conversation goes to the configured hub default worker's `assistant.chat.v1` route.
The chosen video/device target stays independent. Missing advertised capabilities produce
an explicit explanation. A known offline worker retains its durable queued task. Model
responses never claim a tool ran or grant another action; the conversation loop has no
tools. Only the current owner message and at most four clipped ordinary previous turns
(total history <=1024 UTF-8 bytes) are sent. Acknowledgements, service records, files,
phone images and private reasoning are excluded. The complete feed stays in the hub;
only this small context tail is selected for inference.

Only `deepseek-v4.1-flash` is configured. The existing PublicCoding/Go diagnostic remains
separate. General conversation uses the fixed Console Chat Completions endpoint with
explicit Personal/Global authorization, one attempt and disabled thinking. No automatic
route/model fallback. The example profile limits output to 384 tokens; the transport and
reply contracts independently bound bytes. Input reservations conservatively use bytes
plus overhead, not an exact tokenizer. Both ledgers count toward UTC-day/month admission;
unknown calls remain charged across restart. Route limits apply to aggregate usage.

[OpenCode Go](https://opencode.ai/docs/console/go/) documents coding-agent traffic;
[Console inference](https://opencode.ai/docs/console/api/inference/) documents the separate
paid API. Console balance/model permission is required and is not inferred from a Go
subscription. [DeepSeek thinking mode](https://api-docs.deepseek.com/guides/thinking_mode/)
documents the non-thinking parameter. Evidence checked 2026-11-14; actual activation
requires the private worker profile and an explicitly authorized bounded live check.
No real provider call belongs in a test.

## Video generation and honest readiness

`YoutubeGenerate` uses a locally granted generation-only actor in the existing YoutubeBot.
It resolves the configured bound account and checks the dedicated Google/ChatGPT browser
profile before running assets/render. It never calls upload/scheduling. Browser evidence
has distinct ready, login-required, unknown and unavailable states. No cookie extraction,
model-based login guess, automatic consent or lock/challenge bypass is used. A missing
profile or logout reports the relevant account and asks the owner to sign in on that PC.
Unknown state asks for network/browser verification instead of asserting logout.

Acceptance starts background work, not completion. A completed generation requires the
requested count of ready video artifacts; normal actor state/failure notices arrive in
chat. The existing independent media units and operation fences remain. Lost/uncertain
submission responses reconcile status for the same operation, never resubmit automatically.
An interrupted native send reuses its RAM idempotency key when the same request is retried.

## Shared area

Both clients provide an area shared with the other PC's coding agent. The chat command
`/compartilhar instructions` (also `compartilhe com o codex: instructions`) atomically saves
an inert handoff and its acknowledgement, without inference or executing the body. Native
forms support a title/body; MCP also accepts up to eight encrypted transfer UUIDs. Use the
existing device-file client to move bytes; a transfer reference does not turn a received
file into executable source or reveal another device's private inbox.

Tasks contain authenticated provenance, target, state, revision and bounded instructions.
Updates append a bounded note and require the expected revision plus an idempotency key.
The store caps 1000 tasks/40 revisions each; details return the last 20 notes and lists
return compact summaries. Workers see/write only their own target; owner application
credentials may coordinate across the enrolled PCs. Closed tasks stay closed. These are
coordination fences, not exactly-once execution guarantees or execution leases.

The receiving Codex acts when its owner says, for example, "Read the latest task in the
shared area, inspect its instructions and execute within this PC's permissions." It reads
the summary then one body, checks the current owner request/local instructions, records a
revision-fenced claim and reports actual results or blockers. Shared text, model output,
attachments and screenshots remain untrusted data. They cannot authorize external messages,
publication, purchases, credentials or arbitrary device actions. A file is never autoexecuted.

See [MCP setup](../deploy/mcp/README.md) and [native clients](NATIVE_CLIENTS.md).


## 0.5 verification and deployment evidence (2026-11-14)

The repository gate passed 182 Rust tests and 27 Python helper tests, including Clippy,
formatting, documentation/architecture and publication checks. Android unit tests passed
14 with one opt-in live test skipped; two separate standard-age CLI interoperability
checks passed. Lint and the normal APK build passed. Synthetic native visual checks
retain chat navigation and prove desktop encrypted-image appearance/expiry without a
plaintext file. These results are distinct from an actual work-phone capture.

The isolated YoutubeBot checkout preserves the work PC's thumbnail fix. Its integrated
revision is `bec3a3ffc222c9cc9074faa6560fbab00d41aaf9`: 158 Python and 30 Node tests,
extension build/package and publication checks passed. Its
[GitHub Actions run](https://github.com/AlexandreZanata/YoutubeBot/actions/runs/43603001353)
completed successfully. No real video batch, upload or account challenge was performed.

The live hub migrated schema5 to7 after a consistent SQLite backup; all prior table rows
were retained. The local worker migrated schema6 to8; its prior coding ledger, environment,
identity and profiles were preserved. Only the intended Android application gained the
new private-viewer registration scope. The normal APK updated in place to0.5.0 with the
same package, signing certificate and UID; enrollment was not cleared. Phone navigation
acceptance stopped when the owner changed foreground, without further app actions.
The local MCP completed actual stdio initialization/discovery/create/list/read against
the authenticated shared board. An inert work-PC upgrade task is available there.

One explicitly authorized bounded Console request reached the provider through the
normal durable task/worker budget path. The provider returned HTTP402 (balance/billing
permission unavailable). Chat accurately reports that blocker; a successful DeepSeek
conversation is **not** claimed. The call was not automatically retried or switched to Go.
Console billing permission/balance remains an external prerequisite, not a code-test failure.

The existing work PC was reachable, but its actual read-only USB inventory returned zero
devices and its advertised worker was still the older service version. The new work-PC
upgrade is an owner-requested receiving-Codex handoff; delivery does not prove installation.
Actual Google/ChatGPT readiness, generation and the intended phone's messaging/screenshot
acceptance remain to be observed there after the additive update and physical USB setup.
