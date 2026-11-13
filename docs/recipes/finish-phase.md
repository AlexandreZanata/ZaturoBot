# Finish a phase

Check the approved roadmap acceptance gate and stop scope expansion. Verify each claimed
feature is implemented and registered; mark remaining contracts planned with specific
limitations. Review task correctness, security boundaries, resource bounds, dependency
rules, migrations, documentation links, and recovery behavior.

Run focused behavior tests, then `cargo xtask check`. For this foundation, run the local
hub/worker diagnostic demonstration and reproducible memory/load procedure when possible.
Record exact commands, results, environment, actual measurements, and blocked checks.
CI configuration does not imply a remote CI run. A target is not a measurement.

Update components, roadmap, decisions, open questions, and the engineering report. Provide
local run commands, known limitations, unresolved owner decisions, and the next recommended
phase. Do not deploy, push, call paid services, or advance into future phases without the
corresponding authorization.
