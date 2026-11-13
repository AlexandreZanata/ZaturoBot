# Application service instructions

Read the root instructions first. Engine is the single entry point for task and worker use cases. All task transitions, lease checks, acceptance checks, retry limits, and queue limits belong here or pure core rules. Use HubStore repository transactions; never add SQL. Every state transition and its event must commit together. Inject UTC milliseconds into all mutations. Preserve operation IDs across safe diagnostic retries; never generalize this into external-effect replay. Test stale messages and restart behavior before changing recovery.
