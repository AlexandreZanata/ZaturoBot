# Glossary

- Owner: the person authorized to define goals and grant system capabilities.
- Hub / control plane: lightweight task coordination service on the VPS.
- Worker / execution plane: notebook process executing approved typed tools.
- Client plane: interfaces submitting tasks and observing/controlling them.
- Task: durable owner request with lifecycle, execution limits, and acceptance criteria.
- Operation: stable identity of a specific unit of execution across retries/reconnects.
- Assignment / lease: temporary worker ownership with a fencing token and expiry.
- Capability: stable tool permission identifier; a declaration alone is not authorization.
- Idempotency key: caller-provided identity preventing duplicate equivalent task creation.
- Checkpoint: durable evidence sufficient to continue or reconcile work safely.
- Reconciliation: deciding what happened before retrying an uncertain operation.
- Artifact: bounded output referenced by metadata rather than embedded whole in history.
- Broker: future component performing scoped credential-backed operations for an agent.
- Approval: owner authorization bound to a concrete operation, scope, and validity period.
- Untrusted content: external observations that may supply evidence but never permission.
- Implemented: code exists and can be validated; claims still require actual test evidence.
- Planned: documented contract with no claim of a working implementation.
- UTC: canonical persistent time basis. `America/Cuiaba` is the initial owner schedule zone.

Use these English terms consistently in code and documentation. `pt-BR` and `en` are
future owner-interface locale identifiers, not permission to translate engineering files.
