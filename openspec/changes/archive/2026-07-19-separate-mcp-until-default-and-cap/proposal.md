---
change_type: implementation
priority: high
dependencies: []
references:
  - src/mcp.rs
  - tests/mcp_integration.rs
  - openspec/specs/agent-exec-mcp/spec.md
  - openspec/changes/archive/configure-mcp-timeout-budget
---

# Separate MCP Until Default and Cap

**Change Type**: implementation

## Premise / Context

- The archived `configure-mcp-timeout-budget` change made `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` both the omitted `until` default and an explicit-value maximum.
- The implemented maximum currently rejects an MCP tool call when its explicit `until` exceeds the configured value.
- The required contract has two independent host concerns: selecting the omitted default and enforcing a transport-safe cap.
- OpenCode, Hermes, and other MCP hosts need to set these values independently according to their request timeout behavior.
- An explicit `until` above the cap must be rounded down, not rejected, so the managed operation proceeds within the safe request duration.

## Problem / Context

The current environment variable overloads default selection and maximum enforcement. A host cannot preserve or choose an observation default independently from its safety cap. More importantly, rejecting `until` values above the cap requires the agent to recover and retry, while clamping would safely execute the original operation in one request.

## Proposed Solution

Introduce a separate optional environment variable:

`AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS`

Retain `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`, but change its semantics to cap-only.

For MCP `run` and MCP `wait`, calculate:

```text
requested_until = explicit tool until
               or AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS
               or tool legacy default (run=10, wait=30)

effective_until = min(requested_until, AGENT_EXEC_MCP_MAX_UNTIL_SECONDS)
```

The `min` step applies only when the maximum variable is configured. Values equal to or below the maximum remain unchanged. Values above it are silently rounded down to the maximum and the tool proceeds normally.

Both environment variables are optional and independently validated as non-negative integers. An absent default variable preserves the per-tool legacy defaults. An absent maximum variable imposes no cap.

## Acceptance Criteria

- `agent-exec mcp` independently reads optional `AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS` and `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`.
- Omitted `run.until` and `wait.until` use the configured shared default when present; otherwise they retain 10 and 30 seconds respectively.
- Explicit `until` takes precedence over the configured or legacy default.
- A configured maximum caps explicit, configured-default, and legacy-default values using `min`; it does not return an over-maximum error.
- `until=100` with maximum 55 executes with an effective observation duration of 55 seconds.
- A configured default above the maximum is accepted and produces an effective value equal to the maximum.
- Maximum-only configuration caps the legacy defaults when necessary.
- Default-only configuration changes omission behavior without imposing a cap on explicit values.
- Invalid values for either environment variable fail before MCP protocol serving begins and identify the offending variable.
- Clamping does not cancel, kill, or otherwise change detached managed-job lifecycle semantics.

## Explicit Completion Conditions

- `src/mcp.rs` represents default and maximum as independent startup configuration values and calculates one effective `until` for both tools.
- The old over-maximum tool-error path is removed.
- Unit tests cover all precedence combinations, zero values, equal boundary, configured default above maximum, and explicit value above maximum.
- MCP integration tests prove over-maximum `run` creates and observes a real job using the cap, and over-maximum `wait` observes without altering the job.
- Documentation describes the independent environment variables and clamp behavior for OpenCode, Hermes, and other hosts.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Adding MCP timeout/default CLI flags.
- Adding separate run and wait default environment variables.
- Detecting MCP host request deadlines automatically.
- Calculating a safety margin inside agent-exec.
- Changing non-MCP CLI defaults or managed-job cancellation behavior.
