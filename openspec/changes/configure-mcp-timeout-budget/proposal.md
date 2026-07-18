---
change_type: implementation
priority: high
dependencies: []
references:
  - src/main.rs
  - src/mcp.rs
  - tests/mcp_integration.rs
  - openspec/specs/agent-exec-mcp/spec.md
---

# Configure MCP Timeout Budget

**Change Type**: implementation

## Premise / Context

- MCP clients impose platform-specific request deadlines; OpenCode 1.17.18 currently inherits a 60-second MCP SDK request timeout.
- An MCP `wait(until=100)` call can therefore time out near 60 seconds while the managed job continues and completes normally.
- The MCP server currently hard-codes omitted `run.until` to 10 seconds and omitted `wait.until` to 30 seconds in `src/mcp.rs`.
- The timeout budget must be supplied when starting `agent-exec mcp`, because OpenCode, Hermes, and other clients may have different request deadlines.
- `run` and `wait` must share the same configured safety boundary so neither tool can accidentally exceed the client request deadline.

## Problem / Context

The MCP server does not know its client's request timeout. Callers can request an observation period longer than the transport permits, causing an MCP timeout even though the detached managed job remains healthy. Hard-coded defaults also prevent platform integrations from selecting observation windows appropriate to their own timeout budgets.

## Proposed Solution

Add MCP startup configuration that accepts the client request timeout, a safety margin, and separate defaults for `run` and `wait` observation periods. Derive a maximum permitted `until` from the client timeout minus the safety margin.

For both tools:

- An omitted `until` uses its configured tool-specific default.
- An explicit `until` remains supported when it is within the derived maximum.
- An explicit or configured default above the derived maximum is rejected before waiting, with an actionable protocol-safe error.
- Existing behavior remains unchanged when no client timeout configuration is supplied: `run` defaults to 10 seconds and `wait` defaults to 30 seconds, with no new maximum imposed.

The startup surface will support platform configuration such as OpenCode's 60-second deadline with a 5-second margin and 55-second defaults, while allowing Hermes to provide its own measured timeout.

## Acceptance Criteria

- `agent-exec mcp` accepts a client request timeout and safety margin in seconds.
- `agent-exec mcp` accepts separate default `until` values for MCP `run` and MCP `wait`.
- The effective maximum observation period equals client timeout minus safety margin, using checked validation that rejects an invalid or exhausted budget.
- Omitted `run.until` and `wait.until` use their respective configured defaults.
- Explicit `until` values override defaults only when they do not exceed the effective maximum.
- Calls that exceed the effective maximum return immediately with a protocol-safe, actionable error and do not wait for the excessive duration.
- Invalid startup combinations fail through clap or startup validation before the MCP server begins serving.
- With no new options, existing defaults remain `run=10` and `wait=30` and existing MCP clients remain compatible.
- Documentation or help text explains that each platform must pass its own actual MCP request timeout rather than assuming OpenCode's value applies universally.

## Explicit Completion Conditions

- `src/main.rs` exposes and validates the MCP timeout-budget startup options and passes them into `agent_exec::mcp::serve`.
- `src/mcp.rs` stores one validated timeout policy and applies it to both `run` and `wait`.
- Unit tests cover budget derivation, omitted defaults, boundary acceptance, over-budget rejection, and invalid startup combinations.
- MCP integration tests execute both tools with configured defaults and verify over-budget requests return without waiting for the requested duration.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Detecting a client timeout automatically from MCP protocol metadata.
- Changing CLI `run`, `start`, or standalone `wait` defaults outside the MCP server.
- Changing OpenCode, Hermes, or MCP SDK timeout implementations.
- Cancelling or killing a managed job when an MCP observation deadline expires.
