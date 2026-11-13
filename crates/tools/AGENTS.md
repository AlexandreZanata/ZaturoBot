# Typed diagnostic tool

Own deterministic, read-only diagnostic execution. Do not start subprocesses, access
files, read environment variables, or add shell execution. Register each new tool in
the machine-readable registry and document its policy and acceptance checks before use.
Keep the tool specification and strict schemas aligned with the actual typed result.
See [agent tools](../../docs/AGENT_ARCHITECTURE.md).
