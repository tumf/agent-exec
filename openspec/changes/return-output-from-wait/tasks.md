## Implementation Tasks

- [ ] Extend `WaitData` and `src/wait.rs` to return bounded `stdout`/`stderr` excerpts and existing range/total/encoding metadata for terminal and deadline responses. (verification: unit - `cargo test schema::tests::wait_data` asserts the stable output shape and omission rules in `src/schema.rs`)
- [ ] Wire wait output through the existing shared bounded log-reading implementation without changing persisted logs or emitting extra stdout objects. (verification: integration - `cargo test --test integration wait_` exercises real managed jobs through `tests/integration.rs`)
- [ ] Add terminal, non-terminal deadline, stderr, and large-output regression coverage. (verification: integration - targeted tests in `tests/integration.rs` assert output presence, non-terminal exit-code omission, stderr retention, and bounded ranges)
- [ ] Verify HTTP `GET /wait/:id` and MCP `wait` expose completion output through the shared path. (verification: e2e - `cargo test --test serve_integration test_wait_returns_terminal_state` and the wait case in `tests/mcp_integration.rs` assert returned output)
- [ ] Update README, one-minute demo, and agent integration guidance so `wait` is the completion-and-output call while `tail` remains the later/repeated log retrieval call. (verification: integration - `cargo test --test integration wait_returns_json_after_job_finishes` asserts the documented final wait JSON contains command output)

## Final Validation

Expected archive gate: `cflx openspec validate return-output-from-wait --archive-gate`

Run `prek run -a` for repository-wide format, lint, and test verification.
