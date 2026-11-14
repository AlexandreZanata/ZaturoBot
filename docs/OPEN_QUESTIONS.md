# Open owner decisions

These decisions do not prevent the diagnostic foundation. Defaults are conservative
and reversible. Security-critical activation waits for explicit evidence and authorization.

- Q01 — License adoption: recommend MIT OR Apache-2.0. The owner must confirm before
  adding license texts or making a licensed release. Public visibility is not a grant.
- Q02 — Remote topology: resolved for this deployment by ADR-0009: a verified SSH22
  tunnel reaches a loopback hub. A future VPN, TLS proxy, additional devices, or rotation
  API remains a separate scoped decision; no public control endpoint is opened.
- Q03 — Provider eligibility: resolved for the bounded public-coding diagnostic by
  ADR-0010 and dated official research. Only deepseek-v4.1-flash is allowed; paid smoke
  verification is expressly authorized. General non-coding/unattended workloads, new
  data categories, and any retention-policy reliance beyond its date remain unresolved.
- Q04 — Notebook capability: identify CPU, storage, display/server availability, and
  GPU/video encoder support before desktop/video adoption. No GPU assumptions are made.
- Q05 — Unattended unlock: choose a vault key-management and recovery policy; automatic
  reboot unlock trades convenience against theft protection. Vault remains absent.
- Q06 — Side-effect approvals: define exact scopes for GitHub pushes/PRs, Gmail sending,
  upload/scheduling, purchases, and privileged administration. Current diagnostic grants
  authorize none of them.
- Q07 — Retention: choose task/event archival, artifact quotas, encrypted backup destination,
  and restoration objectives. Foundation retains audit history without automated deletion.
- Q08 — Overnight policy: choose local window, daily/monthly budgets, eligible project/task
  classes, notification channel, and stop semantics. No schedule is configured now.
- Q09 — Client priorities: select web/PWA versus native Android and observation transport
  after the API and task engine are stable. No frontend framework is required today.

Record a resolved decision in [decisions](DECISIONS.md), update the canonical contract,
and remove or mark the question resolved with its decision reference.


Resolved for the authorized slice: nativeLinuxRust/Slint, AndroidKotlin, distinct app tokens,
endpoint-encrypted ephemeral desktop phone views, and reuse of YoutubeBot's existing core.
Still pending: physical registered-phone acceptance/unlock behavior, actual Google consent
on the destination OS, migration conflict resolution if destination data already exists,
and any later Android mobile private-screen decoder/full semantic SMS event adapter.
No owner account action or real code content is assumed from a fake-device build test.
