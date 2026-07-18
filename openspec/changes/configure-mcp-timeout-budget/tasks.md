## Implementation Tasks

- [ ] Parse `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` once at MCP startup, accepting a non-negative integer and preserving legacy mode when absent. (verification: unit - tests in `src/mcp.rs` cover absent, zero, valid, empty, malformed, negative, fractional, and overflowing values)
- [ ] Apply the configured value as both omitted default and explicit maximum for MCP `run`, with validation before job creation. (verification: integration - `tests/mcp_integration.rs` proves an omitted value uses the configured bound, an equal boundary is accepted, and an over-maximum call promptly returns an error without creating a job)
- [ ] Apply the configured value as both omitted default and explicit maximum for MCP `wait`, preserving non-cancellation semantics. (verification: integration - `tests/mcp_integration.rs` proves omitted and equal-boundary calls use the configured policy while an over-maximum call promptly errors and leaves the job observable)
- [ ] Preserve legacy MCP behavior when the environment variable is absent: omitted `run.until=10`, omitted `wait.until=30`, and no new maximum for explicit values. (verification: integration - existing and added cases in `tests/mcp_integration.rs` exercise both tools without the environment variable)
- [ ] Document MCP-host configuration, including OpenCode setting `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55` and guidance that Hermes and other hosts must supply their own already-safe value. (verification: manual - inspect the modified repository documentation and confirm it defines no client-timeout, margin, or separate-default setting)
- [ ] Run repository quality gates and resolve failures attributable to this change. (verification: integration - `prek.toml` defines the gates; execute `prek run -a` and require formatting, clippy, and tests to succeed)

## Future Work

- Update external OpenCode and Hermes MCP environment configurations after each host's safe value is selected.

## Final Validation

Expected archive gate: `cflx openspec validate configure-mcp-timeout-budget --archive-gate`
