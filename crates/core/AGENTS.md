# Domain instructions

Read the root instructions first. This crate owns canonical task states, task and
diagnostic contracts, disclosure policy, model evidence, budgets, and pure validation.
It performs no I/O. Persisted times are injected UTC Unix milliseconds.

- `src/lib.rs`: lifecycle, task records, capabilities, and deterministic task validation.
- `src/files.rs`: device file contracts, bounded quotas, safe names and cipher hashes.
- `src/model.rs`: explicit public-coding grants, typed model usage/failures, assisted
  results, reservation identities, and pure request/task/day/month budget policy.
  Personal Go stays classified Personal under its explicit provider opt-in; do not weaken
  the separate public-coding diagnostic disclosure contract or infer free billing.

Never infer a disclosure grant from owner text or model output. Keep legacy diagnostic
JSON readable when extending optional evidence. Extend transition and acceptance tests
with behavioral changes. Do not define wire envelopes, SQL, or authentication here.

`activity.rs` owns non-sensitive daily history/chat contracts and fixed local-day boundaries.
`service.rs` owns explicit allowlisted service requests and capped results. Private-screen
results permit only exact server-known metadata/summaries, never arbitrary text/pixels.
`android.rs` owns explicit USB registration/state/grant contracts; no hardware I/O here.

`intent.rs` recognizes only explicit bounded owner commands; unknown, negated or conditional
text grants no tool. `shared.rs` owns inert handoffs and revision contracts. Personal
conversation context is <=4 ordinary turns/1024 bytes, never private screen content.
Generation requests do not imply publication. Opening SMS is a separate confirmed action.


`computer.rs` owns bounded semantic actions, leases, observations and capture-request recognition. Models never supply approval authority or executable paths.

`intent_router.rs` validates bounded registered proposals against original owner text; `opencode.rs` owns work orders/process/billing evidence and `video_artifact.rs` owns opaque media proof. No model proposal grants additional operations.


The owner authorized0.8 remote private URL and SMS images plus source publication.
Follow docs/REMOTE_PRIVATE_CAPTURE.md (repository root): native URL grant is RAM-only,
private browser profile separate from semantic sessions; SMS verifies unlocked foreground
before/after capture and optional fixed companion return needs its own grant. Never
self-approve capture, copy account profiles or publish private evidence. Preserve the
main-PC administration/same-signature APK boundary.


ADR-0024: a phone print is a current-screen one-shot, never SMS navigation or a send/arrival
watch. Hub fallback returns a native Android hint without model/task execution. One-shot
results require a frame; only explicit watch may be unchanged. Prepared print consumption
requires ready state, focused Hub and unexpired original native confirmation.
