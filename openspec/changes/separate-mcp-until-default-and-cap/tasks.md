## Implementation Tasks

- [ ] Parse `AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS` independently from `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`, with variable-specific startup errors and absent-value compatibility. (verification: unit - `src/mcp.rs` tests cover absent, zero, valid, empty, malformed, negative, fractional, non-Unicode, and overflowing values for both variables)
- [ ] Replace the current maximum-as-default and over-maximum-error behavior with `explicit -> configured default -> legacy tool default`, followed by optional `min(requested, maximum)`. (verification: unit - `src/mcp.rs` table-driven tests cover every precedence row in `design.md`, including default greater than maximum and maximum zero)
- [ ] Wire the resolved value into MCP `run` so over-maximum explicit calls proceed with the capped observation duration and still create a canonical detached job. (verification: integration - `tests/mcp_integration.rs` invokes a real over-cap `run`, asserts a successful `type="run"` envelope and persisted job, and verifies the call returns within the capped bound)
- [ ] Wire the resolved value into MCP `wait` so over-maximum explicit calls proceed with the capped observation duration without changing the managed job. (verification: integration - `tests/mcp_integration.rs` invokes over-cap `wait`, asserts a successful non-terminal `type="wait"` envelope, and confirms subsequent `status` sees the same running job)
- [ ] Preserve legacy behavior when both variables are absent and verify default-only and maximum-only host configurations independently. (verification: integration - `tests/mcp_integration.rs` covers legacy run/wait defaults, uncapped explicit values, default-only omission behavior, and maximum-only clamping of legacy defaults)
- [ ] Update operator documentation to define both environment variables, their precedence, and examples for OpenCode, Hermes, and other MCP hosts. (verification: manual - inspect modified repository documentation and compare examples against the precedence matrix in `openspec/changes/separate-mcp-until-default-and-cap/design.md`)
- [ ] Run repository quality gates and resolve failures attributable to this change. (verification: integration - `prek.toml` defines the gates; execute `prek run -a` and require formatting, clippy, and tests to succeed)

## Future Work

- Update external OpenCode and Hermes MCP environment configurations with host-specific default and maximum values.

## Final Validation

Expected archive gate: `cflx openspec validate separate-mcp-until-default-and-cap --archive-gate`
