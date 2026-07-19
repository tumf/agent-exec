## Implementation Tasks

- [x] Extend `WaitData` and `src/wait.rs` to return bounded `stdout`/`stderr` excerpts and existing range/total/encoding metadata for terminal and deadline responses. (verification: unit - `cargo test schema::tests::wait_data` passes; it asserts the stable output shape and omission rules in `src/schema.rs`)
- [x] Wire wait output through the existing shared bounded log-reading implementation without changing persisted logs or emitting extra stdout objects. (verification: integration - `cargo test --test integration wait_` passes against real managed jobs through `tests/integration.rs`)
- [x] Add terminal, non-terminal deadline, stderr, and large-output regression coverage. (verification: integration - `cargo test --test integration wait_` passes targeted assertions in `tests/integration.rs` for output presence, non-terminal exit-code omission, stderr retention, and bounded ranges)
- [x] Verify HTTP `GET /wait/:id` and MCP `wait` expose completion output through the shared path. (verification: e2e - `cargo test --test serve_integration test_wait_returns_terminal_state` and `cargo test --test mcp_integration mcp_without_until_budget_preserves_legacy_defaults_and_explicit_values` pass with returned-output assertions)
- [x] Update README, one-minute demo, and agent integration guidance so `wait` is the completion-and-output call while `tail` remains the later/repeated log retrieval call. (verification: integration - `cargo test --test integration wait_returns_json_after_job_finishes` passes with final wait-output assertions)

## Final Validation

Expected archive gate: `cflx openspec validate return-output-from-wait --archive-gate`

Run `prek run -a` for repository-wide format, lint, and test verification.
