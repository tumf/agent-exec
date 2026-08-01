## Implementation Tasks

- [x] Define a minimal public embedded client with an explicit jobs root, supervisor executable selection, typed run/status/tail/list/kill inputs and outputs, and stable structured error categories; complete when downstream code needs neither clap argument construction nor JSON serialization/parsing to manage jobs. (verification: unit - public API compile/use cases in `tests/embedded_consumer.rs`; verification-id: embedded-api-local)
- [x] Add an exact reserved-supervisor startup delegation entrypoint that claims only the marker at `argv[1]`, strictly validates generated arguments and pre-created job identity/root, and routes detached launch through the client-selected executable while preserving Unix reaping, Windows Job Object setup, timeout, kill-after, stdin, environment, masking, notifications, logs, and process-tree semantics; complete when delegated supervision outlives the launching consumer and malformed or missing delegation cannot enter consumer handling or launch a workload. (verification: integration - `cargo test --test embedded_consumer`; verification-id: embedded-api-local)
- [x] Add the fixed five-second supervisor startup acknowledgement and launch-failure state transition so `running` is written only by validated delegated supervision after required platform setup, while spawn failure, early exit, validation failure, and acknowledgement timeout persist terminal `failed` without completion notification; complete when late acknowledgement cannot overwrite failure or launch the workload and no-wait still performs this integrity check before returning. (verification: integration - `cargo test --test embedded_consumer`; verification-id: embedded-api-local)
- [x] Refactor run/status/tail/list/kill internals so typed operations perform work and CLI/MCP/HTTP adapters only map inputs and serialize results; specifically remove list's print-only core path and avoid duplicate job enumeration or signal logic. (verification: integration - `cargo test --test embedded_consumer && cargo test --test integration`; verification-id: embedded-api-local)
- [x] Add a separate fixture consumer binary/process that links `agent-exec`, invokes supervisor delegation before its own argument parsing, and proves real run, bounded initial observation, post-launcher status/tail, tag-filtered list recovery, TERM kill, terminal confirmation, and no JSON pollution of consumer stdout; complete when substituting the standalone CLI or a dummy response makes the test fail. (verification: e2e - `cargo test --test embedded_consumer`; verification-id: embedded-api-local)
- [x] Preserve the standalone CLI contract by mapping typed errors back to existing JSON codes, retryability, and exit codes and running existing command and platform tests against the shared typed implementation, including missing/ambiguous job errors, launch failure, default wait, `--no-wait`, raw output bytes, notifications, timeout escalation, supervisor reaping, and Windows-specific compilation/tests; complete when existing integration tests pass unchanged and any available Windows runner executes the embedded delegation fixture rather than compile-checking it only. (verification: integration - `cargo test --test integration && cargo test --all`; verification-id: embedded-api-local)
- [x] Document the embedding startup sequence, reserved invocation ownership, explicit-root requirement, supervisor executable override, sync API, error categories, and lifecycle guarantees without presenting in-process supervision as safe; complete when a minimal consumer example compiles in the embedded-consumer test. (verification: integration - compile the documented example through `tests/embedded_consumer.rs`; verification-id: embedded-api-local)

## Future Work

- Replace the external-command integration in `beads-runner` after this embedded contract is released or available through its chosen source dependency.

## Notes

- Implementation artifacts: `src/embedded.rs` (typed client, request/result types, `JobError`/`JobErrorKind`, `delegate_supervisor_startup`, strict reserved-argument parser), `src/run.rs` (`supervisor_exe` selection, `SupervisorLaunchFailed`, `SUPERVISOR_ACK_TIMEOUT`, `claim_supervisor_ack`/`clear_supervisor_ack`, `await_supervisor_ack`, supervisor-side acknowledgement and failure recording), `src/list.rs` (`list_data`/`list_response` non-printing path), `src/main.rs` (startup delegation before clap; removed the duplicate `_supervise` clap subcommand; `launch_failed` error code), `src/serve.rs` and `src/mcp.rs` (adapters over the typed implementations plus `launch_failed` mapping), `src/start.rs`/`src/restart.rs` (explicit supervisor executable, per-launch marker reset), `tests/fixtures/embedded_consumer.rs` (separate linked consumer binary), `tests/embedded_consumer.rs`, `README.md` (Embedding in a Rust program).
- Launcher/supervisor hand-off uses an atomic `create_new` marker (`supervisor.ack`) inside the job directory, so exactly one side commits: the launcher can never mark a live supervisor failed, and a late supervisor can never resurrect a job the launcher already failed.
- Windows Job Object assignment necessarily follows child spawn, so it stays enforced by the launcher's existing `state.json` handshake, which now runs immediately after the startup acknowledgement.
- The fixture consumer is a `[[bin]]` rather than an example because cargo builds bin targets — but not examples — for `cargo test --test <name>`.
- `tests/embedded_consumer.rs` serializes job launches behind a mutex. Each launch spawns a fresh copy of a large debug binary and must acknowledge within the fixed five-second deadline; letting fourteen tests launch at once made the suite compete with itself for process-startup I/O.
- The only platform-gated test is `supervisor_that_never_acknowledges_fails_on_the_five_second_deadline` (`#[cfg(unix)]`), which needs an executable that hangs regardless of its arguments. Every other embedded test, including the delegation fixture end-to-end run, executes on the CI `windows-latest` matrix entry via `cargo test --all`; that Windows execution has not been observed locally.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-embedded-job-api --archive-gate`

Local verification run on this workspace:
- `cargo fmt --all -- --check` — clean
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo test --test embedded_consumer` — 14 passed
- `cargo test --test integration` — 270 passed, 1 ignored
- `cargo test --test serve_integration` — 24 passed
- `cargo test --test mcp_integration` — 19 passed, 1 ignored
- `cargo test --doc` — 1 passed (the documented embedding example)
- `cargo test --all` — see the run recorded at completion time
