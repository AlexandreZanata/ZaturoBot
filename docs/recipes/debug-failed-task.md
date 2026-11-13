# Debug a failed task

1. Retrieve the task and bounded history using the authenticated CLI/API. Record task,
   operation, worker, lease identity, state, and error category without service tokens.
2. Check worker reachability separately from task state. A disconnect does not prove
   execution failed or that no external effect occurred.
3. Inspect deadline, retry count, capability registration, policy decision, lease expiry,
   and local journal evidence. Follow the crate's tests for the same invariant.
4. Reproduce using a temporary database, fixed time, and synthetic tool/result fixture.
   Do not reuse production accounts or dump environment variables.
5. Fix the owning component and add a regression test. Preserve audit evidence. Run
   focused tests and `cargo xtask check`; report actual failure and validation results.

For uncertain execution, use [interrupted-job recovery](recover-interrupted-job.md)
instead of manually forcing completion or retry.
