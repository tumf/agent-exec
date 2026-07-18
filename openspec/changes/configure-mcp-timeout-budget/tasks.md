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
