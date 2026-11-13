# Change an agent prompt

The bounded diagnostic planner is a registered static trusted prompt. Read the
[prompt declaration contract](../../prompts/README.md) and provider/privacy policies.

Identify the purpose, stable ID/version, permitted variables, trusted instruction source,
untrusted evidence delimiters, and structured output schema. Register the prompt and
keep declared variables exact. Never place secrets in examples or instructions.

Add rendering tests for missing/unknown variables and injection fixtures that attempt to
change permissions or reveal credentials. Run fake-model contract tests and the relevant
behavior evaluation; a snapshot alone cannot prove safe tool dispatch. Update prompt
registry, owning instructions, roadmap, and run `cargo xtask check`. Do not change model
IDs or authorize paid/sensitive-data calls as a hidden part of a prompt edit.
