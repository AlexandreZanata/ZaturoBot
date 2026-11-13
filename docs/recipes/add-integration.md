# Add an integration

Read [integration contracts](../INTEGRATIONS.md), [security](../SECURITY.md), and the
existing project's own instructions first. Do not modify a reference repository.

1. Define the integration ID, typed requests/results, scope, external effects, broker
   permissions, time/resource bounds, and cancellation/reconciliation behavior.
2. Verify official API, terms, authentication, rate limits, and privacy requirements.
3. Register status/path in `config/registry.toml`. Implement only the approved slice.
4. Use a fake remote API or synthetic pipeline fixtures. Test idempotency, uncertain
   timeout, revoked permission, malformed data, redaction, and owner cancellation.
5. Update components, threat model, roadmap, and the canonical integration document.
   Run focused tests and `cargo xtask check` before considering any real-account smoke test.

An adapter being installed does not grant permission to send, upload, purchase, or merge.
