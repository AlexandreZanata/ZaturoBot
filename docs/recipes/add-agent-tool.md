# Add an agent tool

Start with [computer-use tool contract](../COMPUTER_USE.md) and the existing diagnostic.
The policy engine authorizes dispatch; model output only proposes it.

Define a stable ID, description, typed input/output, capability/risk class, resource scope,
timeout, output bound, cancellation behavior, audit fields, and error categories. Identify
external effects and how an intent/result ledger reconciles them. Unknown-effect retries
must stop for reconciliation rather than assume nothing happened.

Register the implementation in the tool catalog and `config/registry.toml`. Test denied
capability, invalid input, output acceptance, timeout/cancellation, restart, and redaction.
Add the smallest executor module; do not introduce an unrestricted command wrapper.
Update components, security/threat documents, and run `cargo xtask check`.
