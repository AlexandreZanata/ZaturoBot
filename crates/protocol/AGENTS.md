# Protocol contracts

Own versioned JSON wire envelopes and shared HTTP response shapes. Domain rules stay in
`zaturo-core`; transport and credentials stay out. Reject unknown message variants and
fields. Add round-trip and malformed-input tests for wire changes. Read
[the protocol contract](../../docs/COMMUNICATION_PROTOCOL.md) before editing.
