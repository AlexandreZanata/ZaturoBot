# Domain instructions

Read the root instructions first. This crate owns canonical task states, task and
diagnostic contracts, disclosure policy, model evidence, budgets, and pure validation.
It performs no I/O. Persisted times are injected UTC Unix milliseconds.

- `src/lib.rs`: lifecycle, task records, capabilities, and deterministic task validation.
- `src/files.rs`: device file contracts, bounded quotas, safe names and cipher hashes.
- `src/model.rs`: explicit public-coding grants, typed model usage/failures, assisted
  results, reservation identities, and pure request/task/day/month budget policy.

Never infer a disclosure grant from owner text or model output. Keep legacy diagnostic
JSON readable when extending optional evidence. Extend transition and acceptance tests
with behavioral changes. Do not define wire envelopes, SQL, or authentication here.

`activity.rs` owns non-sensitive daily history/chat contracts and fixed local-day boundaries.
`service.rs` owns explicit allowlisted service requests and capped results. Private-screen
results permit only exact server-known metadata/summaries, never arbitrary text/pixels.
`android.rs` owns explicit USB registration/state/grant contracts; no hardware I/O here.
