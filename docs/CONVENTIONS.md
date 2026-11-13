# Engineering conventions

Use English, Rust 2024, explicit types, small modules, and `unsafe_code = "forbid"`.
Pin the toolchain and commit `Cargo.lock`. Workspace dependencies and lints belong at
the root; justify dependencies through their concrete responsibility. Avoid empty
abstraction crates and production code that reports success for unimplemented behavior.

Pass UTC time into domain and service operations. Unix timestamp milliseconds are a
persistence representation; owner-local schedules must retain an IANA timezone.
Never use wall-clock sleeps to test domain deadlines. Prefer typed errors with stable
categories. Convert them to bounded public messages at the interface; never include
credentials, request headers, or arbitrary provider bodies.

Keep protocol shapes in `zaturo-protocol`, SQL in `zaturo-db`, business rules in
`zaturo-core`/`zaturo-services`, and authorization in `zaturo-security`. Composition roots
may select and connect implementations, but must not invent a second lifecycle.

`cargo xtask check` runs formatting, warnings-as-errors linting, tests, rustdoc, dependency
rules, and documentation/registry contracts. `cargo xtask arch` and `cargo xtask docs`
provide focused feedback. `cargo xtask secrets` separately validates publication
boundaries over Git-visible work, staging, and history. CI invokes the same entry point. Check failures identify
which canonical source must be updated; do not weaken a guardrail to avoid a failure.

Use descriptive test names for behavioral invariants. Test invalid inputs, stale
messages, and recovery as well as the happy path. Commit messages should explain the
behavioral change and stay reviewable. A migration is append-only after release.
