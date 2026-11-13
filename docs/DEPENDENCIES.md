# Direct dependency rationale

The workspace manifest centralizes dependency declarations; `Cargo.lock` pins resolved
versions and transitive packages. `cargo xtask arch` rejects package-local external
version definitions. Review upstream license notices before distributing binaries.

- `serde` and `serde_json`: typed, bounded JSON domain and wire contracts; strict
  deserialization rejects unsupported inputs.
- `tokio`: two runtime threads per application, asynchronous sockets, timers, signals,
  synchronization, and bounded blocking database work. No per-task runtime is created.
- `axum`: HTTP extraction, routing, and WebSocket upgrade. The hub adds TCP admission,
  idle timeouts, bounded requests, authentication, and body/frame caps.
- `rusqlite`: one mutex-protected SQLite connection per database, explicit transactions,
  and embedded migrations. Bundled SQLite makes builds reproducible and needs a C compiler.
- `tokio-tungstenite` and `futures-util`: outbound worker WebSocket transport and typed
  asynchronous stream/sink operations. Rustls provides remote TLS verification.
- `reqwest`: owner CLI HTTP calls and validated origin parsing at the composition roots.
  Disable default features and select Rustls; do not depend on a system OpenSSL runtime.
- `url`: canonical origin-policy parsing in the security crate, shared by both apps.
- `clap`: typed, discoverable CLI arguments and help output.
- `toml`: strict app configuration and repository-contract manifests.
- `uuid`: random task, operation, lease, and default idempotency identifiers.
- `sha2` and `subtle`: fixed-size credential digests and constant-time digest comparison.
  These bootstrap credentials are not an encrypted personal vault.
- `thiserror` and `anyhow`: typed library failures and composition-root context.
  External error responses expose categories rather than internal strings or secrets.
- `tracing` and `tracing-subscriber`: structured JSON events and operator-selected log levels.
- `tempfile`: isolated test databases and synthetic local state; never production fixtures.

Transitive dependencies are inherited from these adapters, pinned in the lockfile, and
compiled with the toolchain recorded in `rust-toolchain.toml`. This inventory is not a
claim that a vulnerability or license audit has been performed. Python 3 is an optional
benchmark dependency only; the runtime and repository quality gate are Rust-based.
