---
change_type: implementation
priority: high
dependencies: []
references:
  - src/mcp.rs
  - tests/mcp_integration.rs
  - openspec/specs/agent-exec-mcp/spec.md
---

# Configure MCP Maximum Until

**Change Type**: implementation

## Premise / Context

- MCP clients impose platform-specific request deadlines; OpenCode 1.17.18 currently times out MCP tool requests after 60 seconds.
- An MCP `wait(until=100)` call can therefore time out near 60 seconds while the detached managed job continues and completes normally.
- OpenCode, Hermes, and other hosts must choose their own safe observation value because their request deadlines differ.
- The MCP server currently hard-codes omitted `run.until` to 10 seconds and omitted `wait.until` to 30 seconds in `src/mcp.rs`.
- The host should pass one already-safe value through the MCP process environment; agent-exec should not model the host timeout or calculate a safety margin.

## Problem / Context

The MCP server cannot infer its client's request deadline. Explicit or default observation periods can outlive the transport request, causing the client to lose the response even though the managed job remains healthy. Adding separate CLI defaults, client-timeout settings, and margin settings would duplicate one host integration decision across several controls.

## Proposed Solution

Support one optional environment variable:

`AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`

When set to a valid non-negative integer, its value is both:

- the omitted `until` default for MCP `run` and MCP `wait`; and
- the maximum accepted explicit `until` for both tools.

A value above this maximum is rejected before waiting. For MCP `run`, rejection occurs before job creation. For MCP `wait`, rejection leaves the existing job untouched.

When the environment variable is absent, preserve the existing behavior: omitted `run.until` uses 10 seconds, omitted `wait.until` uses 30 seconds, and no new maximum is imposed.

OpenCode can pass `55` for its current 60-second request deadline. Hermes and other hosts pass their independently selected safe value.

## Acceptance Criteria

- `agent-exec mcp` reads optional `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` from its process environment.
- A configured value becomes the shared omitted `until` default and explicit `until` maximum for MCP `run` and MCP `wait`.
- Explicit values equal to the configured maximum are accepted.
- Explicit values above the configured maximum return immediately with a protocol-safe, actionable error.
- Invalid environment values fail before the MCP server begins serving.
- Over-budget MCP `run` calls create no job; over-budget MCP `wait` calls do not signal or alter the job.
- With the variable absent, existing defaults remain `run=10` and `wait=30`, and explicit values retain existing behavior.
- Documentation explains that each MCP host supplies an already-safe value; agent-exec performs no client-timeout or margin calculation.

## Explicit Completion Conditions

- `src/mcp.rs` parses and validates the environment variable once at MCP startup and applies the resulting policy to both tools.
- Unit tests cover absent, zero, valid, malformed, boundary, and over-maximum values.
- MCP integration tests verify shared omitted defaults and prompt over-maximum rejection for both tools.
- Operator documentation includes environment examples for OpenCode and platform-neutral guidance for Hermes and other hosts.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Adding MCP timeout/default CLI flags.
- Accepting separate defaults for `run` and `wait`.
- Accepting a client request timeout or safety-margin setting.
- Detecting a client timeout automatically from MCP protocol metadata.
- Changing CLI `run`, `start`, or standalone `wait` defaults outside the MCP server.
- Changing OpenCode, Hermes, or MCP SDK timeout implementations.
- Cancelling or killing a managed job when an MCP observation deadline expires.
