## Implementation Tasks

- [ ] Define a minimal public embedded client with an explicit jobs root, supervisor executable selection, typed run/status/tail/list/kill inputs and outputs, and stable structured error categories; complete when downstream code needs neither clap argument construction nor JSON serialization/parsing to manage jobs. (verification: unit - public API compile/use cases in `tests/embedded_consumer.rs`; verification-id: embedded-api-local)
- [ ] Add an exact reserved-supervisor startup delegation entrypoint and route detached launch through the client-selected executable, preserving Unix reaping, Windows Job Object handshake, timeout, kill-after, stdin, environment, masking, notifications, logs, and process-tree semantics; complete when delegated supervision outlives the launching consumer and missing delegation fails without a false running record. (verification: integration - `cargo test --test embedded_consumer`; verification-id: embedded-api-local)
- [ ] Refactor run/status/tail/list/kill internals so typed operations perform work and CLI/MCP/HTTP adapters only map inputs and serialize results; specifically remove list's print-only core path and avoid duplicate job enumeration or signal logic. (verification: integration - `cargo test --test embedded_consumer && cargo test --test integration`; verification-id: embedded-api-local)
- [ ] Add a separate fixture consumer binary/process that links `agent-exec`, invokes supervisor delegation before its own argument parsing, and proves real run, bounded initial observation, post-launcher status/tail, tag-filtered list recovery, TERM kill, terminal confirmation, and no JSON pollution of consumer stdout; complete when substituting the standalone CLI or a dummy response makes the test fail. (verification: e2e - `cargo test --test embedded_consumer`; verification-id: embedded-api-local)
- [ ] Preserve the standalone CLI contract by running existing command and platform tests against the shared typed implementation, including missing/ambiguous job errors, default wait, `--no-wait`, raw output bytes, notifications, timeout escalation, supervisor reaping, and Windows-specific compilation/tests where available. (verification: integration - `cargo test --test integration && cargo test --all`; verification-id: embedded-api-local)
- [ ] Document the embedding startup sequence, reserved invocation ownership, explicit-root requirement, supervisor executable override, sync API, error categories, and lifecycle guarantees without presenting in-process supervision as safe; complete when a minimal consumer example compiles in the embedded-consumer test. (verification: integration - compile the documented example through `tests/embedded_consumer.rs`; verification-id: embedded-api-local)

## Future Work

- Replace the external-command integration in `beads-runner` after this embedded contract is released or available through its chosen source dependency.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-embedded-job-api --archive-gate`
