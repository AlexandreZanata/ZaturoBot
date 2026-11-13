# Add a crate

1. Search [components](../COMPONENTS.md) and the workspace for an existing owner.
   Introduce a crate only when a concrete responsibility needs a separate boundary.
2. Record the reason and allowed dependencies in [decisions](../DECISIONS.md).
3. Add a workspace member using centralized dependencies/lints and Rust 2024.
4. Add its path and allowed internal edges to `architecture.toml`; do not add a cycle.
5. Write crate `AGENTS.md` with responsibility, files, non-responsibilities, and tests.
6. Register the package in [components](../COMPONENTS.md), add meaningful behavioral
   tests, and run `cargo xtask arch` followed by `cargo xtask check`.

A directory for planned work is not a reason to create an empty Rust package.
