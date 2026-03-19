## Implementation Tasks

- [ ] Update the supervisor exit path in `src/run.rs` so terminal `state.json` is written immediately after `child.wait()` returns, before log-thread joins or completion-sink delivery (verification: `src/run.rs` persists terminal `JobState` before follow-up cleanup steps).
- [ ] Make post-exit cleanup non-blocking for job-state correctness so inherited stdout/stderr from descendants cannot keep `status`/`wait` in `running` (verification: `src/run.rs`, `src/status.rs`, and `src/wait.rs` still expose terminal state once the wrapped root process exits).
- [ ] Add an integration regression test in `tests/integration.rs` that reproduces a parent process exiting while a descendant keeps inherited stdio open, and assert `status` becomes terminal promptly (verification: new test in `tests/integration.rs` fails before the fix and passes after it).
- [ ] Add an integration assertion that `_supervise` does not linger after the reproduced command completes (verification: `tests/integration.rs` checks process behavior using the existing harness and allowed platform assumptions).

## Future Work

- Consider a separate cleanup/reconciliation path for stale job directories left behind by pre-fix versions.
