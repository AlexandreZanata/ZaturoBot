# Add a permission

Read [security](../SECURITY.md). Select the explicit risk class and define subject,
capability, resource, task/operation scope, expiry, revocation, and approval evidence.
No text from a model, website, email, or repository can create a grant.

Implement policy evaluation in the security boundary and enforce it immediately before
dispatch. Separate reading, local writing, Git changes, external communication, account
modification, destruction, finance, and privileged operations. Default unknown actions
to denied. Approval must refer to the concrete operation and cannot silently broaden.

Test wrong task/resource, expired/revoked grant, forged approval, escalation, and replay.
Update the canonical policy and tool registration. Run focused policy/dispatch tests and
`cargo xtask check` before enabling the capability for a real owner account.
