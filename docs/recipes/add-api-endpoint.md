# Add an API endpoint

1. Define the endpoint's authenticated role, bounded input/output, error categories,
   idempotency behavior, and pagination in [protocol](../COMMUNICATION_PROTOCOL.md).
2. Reuse or add the use case in `zaturo-services`. SQL stays in `zaturo-db`; handlers
   decode, authorize, invoke, and encode rather than owning lifecycle rules.
3. Add typed wire shapes in `zaturo-protocol` when shared. Reject unsupported versions,
   unknown privileged behavior, and oversized input before expensive work.
4. Test unauthenticated/wrong-role requests, invalid input, success, domain conflicts,
   storage failure where meaningful, and response bounds with a loopback/local router.
5. Update components and client examples, then run focused tests and `cargo xtask check`.

Never expose an unauthenticated control endpoint because it is only used by a dashboard.
