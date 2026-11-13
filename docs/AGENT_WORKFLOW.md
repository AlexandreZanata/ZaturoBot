# Coding-agent workflow

Start with root `AGENTS.md`. Identify requested behavior, consult components, read the
affected crate instructions and decisions, search for reusable code, and choose the
smallest safe change. List the affected contract and tests before implementation.

During work, keep interfaces explicit and modules focused. Change only necessary files.
Add tests for the invariant and its failure paths. Record dependency and security
choices. Update canonical documentation in the same change; do not copy another
component's definition into a new config or prompt. Mark incomplete features planned.

After focused tests pass, run `cargo xtask check`. Review the diff for secrets, accidental
scope expansion, fake success paths, unbounded allocations, and cross-layer SQL or policy.
Summarize actual behavior, commands run, results, known limitations, and remaining owner
decisions. Never claim a command, benchmark, or integration worked without evidence.

## Interrupted development

Read `git status`, existing changes, the roadmap, relevant decisions, and previous test
results before resuming. Preserve owner edits. Record the last verified state and the
next failing or pending check. Avoid rerunning destructive external operations merely
because the conversation context was lost. Runtime job recovery follows
[recover-interrupted-job](recipes/recover-interrupted-job.md), a separate concern.

Codex, Claude Code, and OpenCode use the same canonical instructions. `CLAUDE.md` is a
pointer, not a parallel rulebook. Any adapter for a future coding agent must preserve
the repository's authorization and evidence requirements.
