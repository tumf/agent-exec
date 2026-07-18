## Implementation Tasks

- [ ] Add MCP startup options for client timeout, safety margin, default run `until`, and default wait `until`; preserve current defaults when timeout configuration is absent. (verification: unit - tests in `src/main.rs` prove clap parsing accepts configured and omitted values and rejects invalid combinations)
- [ ] Implement a validated MCP timeout policy that derives the maximum observation duration with checked arithmetic and produces actionable errors for exhausted budgets or defaults above the maximum. (verification: unit - tests in `src/mcp.rs` cover equal-to-limit acceptance, over-limit rejection, margin greater than or equal to timeout, and legacy unbounded mode)
- [ ] Wire the configured run default and maximum into the MCP `run` tool without changing detached job lifecycle semantics. (verification: integration - `tests/mcp_integration.rs` starts an MCP server with a custom policy, omits `run.until`, and observes the configured bounded behavior)
- [ ] Wire the configured wait default and maximum into the MCP `wait` tool while preserving non-cancellation semantics. (verification: integration - `tests/mcp_integration.rs` omits `wait.until`, verifies the configured bound, and confirms the job remains running after the observation deadline)
- [ ] Reject explicit `run.until` and `wait.until` values above the client-safe maximum before entering the wait path. (verification: integration - `tests/mcp_integration.rs` proves over-budget calls return promptly, `run` creates no job, and `wait` leaves its job observable)
- [ ] Update CLI help and operator-facing documentation with platform-specific examples, including OpenCode's measured 60-second timeout configured with a 5-second margin, while instructing Hermes and other integrations to pass their own measured timeout. (verification: manual - run `cargo run --bin agent-exec -- mcp --help` and inspect the modified repository documentation for correct precedence and platform-neutral wording)
- [ ] Run repository quality gates and resolve failures attributable to this change. (verification: integration - `prek.toml` is the repository evidence; execute `prek run -a` and require formatting, clippy, and tests to succeed)

## Future Work

- Update external OpenCode and Hermes deployment configurations after the corresponding platform timeout values are measured and approved.

## Final Validation

Expected archive gate: `cflx openspec validate configure-mcp-timeout-budget --archive-gate`
