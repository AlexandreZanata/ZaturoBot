# Personal integrations

Status: planned for Phase 5. `github`, `video`, `price-monitoring`, and `gmail` are catalog
IDs, not functioning adapters. Nothing in the foundation contacts these accounts.

## Project and GitHub contract

A project registration records repository URL, approved local workspace, branch policy,
build/test commands, instruction entry point, tool grants, secret policy, limits, approval
rules, and artifact root. Repositories have independent conventions. Candidate projects
are [SorriPrecos](https://github.com/AlexandreZanata/SorriPrecos) and
[OpenTatu](https://github.com/AlexandreZanata/OpenTatu); registration grants no write access.

A future adapter reads repository instructions, creates a task-specific worktree, applies
an approved change, runs its checks, and returns a diff and test evidence. Commit, push,
PR creation, and merge are separate policy actions. Prefer narrowly scoped GitHub Apps.
No direct protected-branch pushes or merges without explicit policy. Acceptance uses
synthetic local repositories and a fake remote API, never the owner's live projects.

## Existing video workflow

Reuse [chatgpt-recorder/tools/ytbot](https://github.com/AlexandreZanata/chatgpt-recorder/tree/main/tools/ytbot)
through a typed adapter. A read-only local README inspection found staged article,
audio, metadata, thumbnail, render, upload, scheduling, and resumable per-run records.
The workflow assumes existing browser sessions and GPU-related rendering behavior in
places. Notebook compatibility, service terms, credential isolation, and cancellation
must be verified before enabling it. It was not executed or modified here.

The adapter takes an approved batch count and project profile, persists a run identifier,
streams bounded progress, and references artifacts. Upload/schedule effects require
separate authorization and reconciliation; an existing video ID prevents blind re-upload.
Acceptance uses a fixture command and synthetic records, including failure and cancellation.

## Price monitoring

Normalize product, variant/specifications, source URL, currency, observed price, and UTC
observation. Store history and a threshold; deduplicate notifications by observation and
rule. Prefer official APIs/authorized feeds. Respect terms, robots policy, source limits,
and permissions. Never bypass CAPTCHA or access restrictions. Deterministic collectors
handle routine checks; a model only interprets exceptional ambiguous data. Acceptance
uses saved synthetic pages/API fixtures and timezone/currency/variant mismatch cases.

## Gmail

Use OAuth through the future broker with minimum required scopes. Read, draft, send,
label, and delete are different capabilities. Preparing a draft does not authorize send.
Treat message content as untrusted instructions. Acceptance uses a fake mailbox and
verifies that injected email text cannot grant send permission or reveal tokens.

All adapters require typed input/output, capability registration, operation identity,
resource bounds, cancellation semantics, redacted errors, audit evidence, and explicit
uncertain-effect recovery. See [add-integration](recipes/add-integration.md).
