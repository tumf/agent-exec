## Implementation Tasks

- [x] Extend `WaitData` and `src/wait.rs` to return bounded `stdout`/`stderr` excerpts and existing range/total/encoding metadata for terminal and deadline responses. (verification: unit - `cargo test schema::tests::wait_data` passes; it asserts the stable output shape and omission rules in `src/schema.rs`)
- [x] Wire wait output through the existing shared bounded log-reading implementation without changing persisted logs or emitting extra stdout objects. (verification: integration - `cargo test --test integration wait_` passes against real managed jobs through `tests/integration.rs`)
- [x] Add terminal, non-terminal deadline, stderr, and large-output regression coverage. (verification: integration - `cargo test --test integration wait_` passes targeted assertions in `tests/integration.rs` for output presence, non-terminal exit-code omission, stderr retention, and bounded ranges)
- [x] Verify HTTP `GET /wait/:id` and MCP `wait` expose completion output through the shared path. (verification: e2e - `cargo test --test serve_integration test_wait_returns_terminal_state` and `cargo test --test mcp_integration mcp_without_until_budget_preserves_legacy_defaults_and_explicit_values` pass with returned-output assertions)
- [x] Update README, one-minute demo, and agent integration guidance so `wait` is the completion-and-output call while `tail` remains the later/repeated log retrieval call. (verification: integration - `cargo test --test integration wait_returns_json_after_job_finishes` passes with final wait-output assertions)

## Final Validation

Expected archive gate: `cflx openspec validate return-output-from-wait --archive-gate`

Run `prek run -a` for repository-wide format, lint, and test verification.

## Acceptance #1 Failure Follow-up
- [x] `src/run.rs:1543-1585` は terminal state をログ drain 前に永続化し、`src/wait.rs:80-85` は terminal 観測直後にログを返すため、最終 stdout/stderr を取りこぼし得ます。bounded drain 完了後の応答保証と競合回帰テストが必要です。 (verification: integration - `cargo test --test integration wait_returns_output_after_root_process_exits_before_pipe_drain -- --nocapture` passes)
- [x] `src/schema.rs:235-255` では wait の出力・encoding・range・total fields が必須ですが、公開 `schema/agent-exec.schema.json:368-405` の `WaitResponse` に定義されていません。公開 Schema と実応答を同期し、Schema 検証テストを追加してください。 (verification: integration - `cargo test --test integration schema_wait_response_matches_wait_output_contract -- --nocapture` passes)

## Acceptance #2 Failure Follow-up
- [x] 公開 Schema に `created` state を追加し、実際の created/running deadline 応答および terminal wait 応答全体を Schema 検証する回帰テストを追加した。 (verification: integration - `cargo test --test integration schema_validates_actual_wait_responses -- --nocapture` passes)
- [x] kill 経路が terminal state と `logs_drained` を先行永続化しないようにし、TERM trap の最終出力を wait が返す回帰テストを追加した。 (verification: integration - `cargo test --test integration wait_ -- --nocapture` passes)
- [x] HTTP `GET /wait/:id` を shared drain-aware `wait_response` 経路へ統一し、terminal-before-drain の stdout/stderr 回帰テストを追加した。 (verification: e2e - `cargo test --test serve_integration test_wait_returns_output_after_terminal_before_drain -- --nocapture` passes)
- [x] deadline 分岐を非terminal state のみに制限し、terminal state は `logs_drained` 完了まで待機するようにした。 (verification: integration - `cargo test --test integration wait_ -- --nocapture` passes)

## Acceptance #3 Failure Follow-up
- [x] `WaitResponse.state` に `created` を追加し、`create` 済み job の `wait --until 0` 応答全体を公開 Schema で検証する回帰テストを追加した。 (verification: integration - `cargo test --test integration schema_validates_actual_wait_responses -- --nocapture` passes)
