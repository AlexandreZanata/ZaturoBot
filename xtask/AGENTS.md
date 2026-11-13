# Repository validation

Read the root [AGENTS.md](../AGENTS.md) first. This crate owns repository checks and
runs the exact validation CI uses. `architecture.toml` owns permitted internal crate
edges; `config/registry.toml` owns tool, integration, migration, prompt and feature
registration. Check actual manifests and files rather than maintaining another copy
of those contracts here. Fail with actionable paths and never edit files during a
check. Test a rejection whenever extending a contract check.

Run `cargo xtask docs`, `cargo xtask arch`, or `cargo xtask check --locked` from the
repository root. Checks may spawn Cargo only from command dispatch, never tests.
