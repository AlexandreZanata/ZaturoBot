# Open owner decisions

These decisions do not prevent the diagnostic foundation. Defaults are conservative
and reversible. Security-critical activation waits for explicit evidence and authorization.

- Q01 — License adoption: recommend MIT OR Apache-2.0. The owner must confirm before
  adding license texts or making a licensed release. Public visibility is not a grant.
- Q02 — Remote topology: choose VPN-only access or a private TLS proxy, DNS/certificates,
  device ACLs, and service-token provisioning/rotation. No production deployment authorized.
- Q03 — Provider eligibility: verify Go's current coding-workload restrictions and each
  data-category/privacy setting. Decide whether general non-coding tasks need another
  provider. Candidate model IDs remain configuration choices, not guaranteed capabilities.
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
