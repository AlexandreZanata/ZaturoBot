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
- `reqwest`: owner CLI HTTP calls, validated origin parsing at the composition roots,
  and the worker-local bounded OpenCode Go adapter.
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
- `age`: standard streaming X25519 recipient encryption and authenticated decryption.
  Native private identities remain on endpoints; no custom encryption construction.
  Default features are disabled; plugin/SSH identity execution is not enabled.
- `libc`: the `O_NOFOLLOW` constant for safe Unix file opening; no unsafe code.
- `tempfile`: isolated test databases and synthetic local state; never production fixtures.

Transitive dependencies are inherited from these adapters, pinned in the lockfile, and
compiled with the toolchain recorded in `rust-toolchain.toml`. This inventory is not a
claim that a vulnerability or license audit has been performed. Python 3 supports optional benchmarks and the Linux enrollment installer; the hub/worker are Rust-based and the Rust quality driver also invokes hermetic Python
deployment-helper fixtures.


Native/service additions: `chrono` validates UTC/local-day boundaries; `rustix` provides
safe owned process-group signalling; `zeroize` wipes owned sensitive buffers. `slint`,
`slint-interpreter` and `slint-build` provide the pinned native Linux software renderer,
UI syntax checks and event lifecycle without weakening first-party unsafe-code policy.
`image` is PNG-only for bounded memory decoding, not a network fetch path. Android uses
native Kotlin/Android APIs, Keystore AES-GCM and pinned SSHJ with canonical SSH host-key
verification; Gradle/Maven manifests pin its build dependencies. The optional design-PDF
generator uses reportlab/Pillow outside the running hub/worker. Provider policy is unchanged.

Android presentation also bundles a small pinned subset of Google's Material Symbols
Rounded as Android vector resources under Apache-2.0. Upstream paths, revision, hashes,
license and notice ship with the app; see [icon provenance](../mobile/android/docs/ICONS.md).
No Material Components runtime dependency or external icon/font fetch is required.
