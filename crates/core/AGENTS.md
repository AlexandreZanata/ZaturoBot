# Domain instructions

Read the root instructions first. This crate owns canonical task states, task and diagnostic contracts, budgets, and pure validation. It performs no I/O. Persisted times are injected UTC Unix milliseconds. Extend state-transition tests whenever legal transitions change. Do not define wire envelopes, SQL, or authentication here.
