## Implementation Tasks

- [x] Parse `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` once at MCP startup, accepting a non-negative integer and preserving legacy mode when absent. (verification: unit - `cargo test mcp::` passes; tests in `src/mcp.rs` cover absent, zero, valid, empty, malformed, negative, fractional, and overflowing values)
- [x] Apply the configured value as both omitted default and explicit maximum for MCP `run`, with validation before job creation. (verification: integration - `cargo test --test mcp_integration` passes; `tests/mcp_integration.rs` proves an omitted value uses the configured bound, an equal boundary is accepted, and an over-maximum call promptly returns an error without creating a job)
- [x] Apply the configured value as both omitted default and explicit maximum for MCP `wait`, preserving non-cancellation semantics. (verification: integration - `cargo test --test mcp_integration` passes; `tests/mcp_integration.rs` proves omitted and equal-boundary calls use the configured policy while an over-maximum call promptly errors and leaves the job observable)
- [x] Preserve legacy MCP behavior when the environment variable is absent: omitted `run.until=10`, omitted `wait.until=30`, and no new maximum for explicit values. (verification: integration - `cargo test --test mcp_integration` passes; existing and added cases in `tests/mcp_integration.rs` exercise both tools without the environment variable)
- [x] Document MCP-host configuration, including OpenCode setting `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55` and guidance that Hermes and other hosts must supply their own already-safe value. (verification: manual - `src/main.rs:561-577` exposes MCP host startup guidance; `README.md:731-739` documents the shared budget; run `cargo run -- mcp --help`)
- [x] Run repository quality gates and resolve failures attributable to this change. (verification: integration - `prek.toml` defines the gates; `CARGO_TARGET_DIR=/var/folders/dg/xh2k12k51yb300kdz4xmtr7m0000gn/T/opencode/agent-exec-configure-mcp-timeout-budget-target prek run -a` passed)

## Future Work

- Update external OpenCode and Hermes MCP environment configurations after each host's safe value is selected.

## Final Validation

Expected archive gate: `cflx openspec validate configure-mcp-timeout-budget --archive-gate`

## Acceptance #1 Failure Follow-up
- [x] `openspec/changes/configure-mcp-timeout-budget/specs/agent-exec-mcp/spec.md:27` は無効な環境変数を stderr で識別することを要求するが、`src/mcp.rs:16-20` のエラーは `src/main.rs:740-779` の通常 CLI エラー境界に渡り、実行確認では stdout に JSON エラーを出し stderr は空だった。MCP 起動エラーを stderr に出し、プロトコル serving 前に終了する実装と統合テストを追加すること。 (verification: integration - `cargo test --test mcp_integration` passes; `mcp_invalid_until_budget_fails_before_serving_and_reports_to_stderr` verifies stderr-only startup failure)
- [x] `openspec/changes/configure-mcp-timeout-budget/tasks.md:7` の完了証拠が事実と一致しない。`src/main.rs:560-561` と実際の `cargo run -- mcp --help` は環境変数を案内せず、`README.md:731-739` は `config.toml` の説明である。実在する文書パスを証拠として記載するか、指定箇所へ MCP observation budget の案内を追加すること。 (verification: manual - `src/main.rs:560-564` help text and `README.md:731-737` document the host-selected MCP observation budget)

## Acceptance #2 Failure Follow-up
- [x] `src/mcp.rs:27-31` は `std::env::var(...).ok()` により非UTF-8の `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` を未設定として扱う。実行確認では値 `0xff` で MCP serving に進み、stdout に通常CLI JSONエラーを出して stderr は空だった。これは無効値を serving 前に拒否し stderr で識別する `openspec/changes/configure-mcp-timeout-budget/specs/agent-exec-mcp/spec.md:5,22-27` に違反する。`var_os` 等で「未設定」と「非UTF-8」を区別して起動エラーにし、`tests/mcp_integration.rs:88-100` に非UTF-8環境値の統合テストを追加すること。 (verification: `cargo test mcp::` and `cargo test --test mcp_integration` passed; the Unix-only integration test verifies a `0xff` value fails before serving with empty stdout and an identifying stderr diagnostic)
