## Implementation Tasks

- [x] Parse `AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS` independently from `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`, with variable-specific startup errors and absent-value compatibility. (verification: unit - `src/mcp.rs` tests cover absent, zero, valid, empty, malformed, negative, fractional, non-Unicode, and overflowing values for both variables)
- [x] Replace the current maximum-as-default and over-maximum-error behavior with `explicit -> configured default -> legacy tool default`, followed by optional `min(requested, maximum)`. (verification: unit - `src/mcp.rs` table-driven tests cover every precedence row in `design.md`, including default greater than maximum and maximum zero)
- [x] Wire the resolved value into MCP `run` so over-maximum explicit calls proceed with the capped observation duration and still create a canonical detached job. (verification: integration - `tests/mcp_integration.rs` invokes a real over-cap `run`, asserts a successful `type="run"` envelope and persisted job, and verifies the call returns within the capped bound)
- [x] Wire the resolved value into MCP `wait` so over-maximum explicit calls proceed with the capped observation duration without changing the managed job. (verification: integration - `tests/mcp_integration.rs` invokes over-cap `wait`, asserts a successful non-terminal `type="wait"` envelope, and confirms subsequent `status` sees the same running job)
- [x] Preserve legacy behavior when both variables are absent and verify default-only and maximum-only host configurations independently. (verification: integration - `tests/mcp_integration.rs` covers legacy run/wait defaults, uncapped explicit values, default-only omission behavior, and maximum-only clamping of legacy defaults)
- [x] Update operator documentation to define both environment variables, their precedence, and examples for OpenCode, Hermes, and other MCP hosts. (verification: manual - source path: `src/mcp.rs`; documentation: `README.md:731-740`)
- [x] Run repository quality gates and resolve failures attributable to this change. (verification: manual - runnable command: `cargo test --all`)

## Future Work

- Update external OpenCode and Hermes MCP environment configurations with host-specific default and maximum values.

## Final Validation

Expected archive gate: `cflx openspec validate separate-mcp-until-default-and-cap --archive-gate`

## Acceptance #1 Failure Follow-up
- [x] Replace non-verifiable documentation and quality-gate verification notes with repository paths and a runnable command. (verification: manual - source path: `src/mcp.rs`; test file: `tests/mcp_integration.rs`)
- [x] Reject an out-of-range MCP `until` before job creation and preserve protocol availability. (verification: unit - `src/mcp.rs` rejects 2^64; integration - `tests/mcp_integration.rs` rejects it without creating a job, then completes a valid run)

## Acceptance #2 Failure Follow-up
- [x] Reclassify verification ownership using supported categories while retaining repository path and runnable-command evidence. (verification: `src/mcp.rs`; `tests/mcp_integration.rs`; `README.md:731-740`)

## Acceptance #3 Failure Follow-up
- [x] Replace archive-gate task evidence with implemented MCP configuration, integration coverage, and operator documentation. (verification: `src/mcp.rs`; `tests/mcp_integration.rs`; `README.md:731-740`)
- [x] `AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS=18446744073709551615` が起動時に受理されるが、until 省略の MCP `run` で `src/mcp.rs:94-138` から `src/run.rs:740` に値が渡り、`overflow when adding duration to instant` で panic する。実行可能範囲外の環境値を protocol serving 前に変数名付きで拒否し、default/max 両方の境界テストを追加すること。 (verification: unit - `src/mcp.rs`; integration - `tests/mcp_integration.rs`; command: `cargo test mcp`)

## Acceptance #5 Failure Follow-up
- [x] Rewrite completed task evidence to cite the implemented MCP configuration, integration tests, and README rather than archive validation. Keep archive validation only in the non-checkbox `## Final Validation` section. (verification: integration - source path: `src/mcp.rs`; test file: `tests/mcp_integration.rs`; documentation: `README.md:731-740`)
- [x] Preserve the end-of-file formatting correction and rerun the repository quality gate. (verification: manual - runnable command: `prek run -a`; source path: `src/mcp.rs`)
