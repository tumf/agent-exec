## Context

`agent-exec mcp` synchronously serves tool calls while managed jobs execute independently under detached supervision. The MCP client controls the outer JSON-RPC request deadline. A tool observation period longer than that deadline loses the response even though the job continues normally.

## Decision

Read one optional process environment variable at MCP startup:

`AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`

When configured, the parsed seconds value is one server-level policy reused by both MCP observation tools:

- omitted `run.until` uses the configured value;
- omitted `wait.until` uses the configured value;
- explicit `run.until` and `wait.until` must not exceed the configured value.

The MCP host is responsible for choosing an already-safe value. Agent-exec does not accept the host's raw request timeout and does not calculate or configure a safety margin.

## Precedence

1. A per-call `until` value overrides omission behavior but must fit within the configured maximum.
2. When `until` is omitted and the environment variable is configured, the configured maximum is also the default.
3. When the environment variable is absent, legacy defaults remain `run=10` and `wait=30`, with no new maximum.

## Validation and Error Behavior

The environment variable is parsed once before stdio protocol serving. Empty, malformed, negative, fractional, or overflowing values fail startup with a diagnostic on stderr.

A per-call value above the maximum returns a protocol-safe tool error containing the requested value and permitted maximum. It does not enter the observation path, cancel the MCP server, or signal a managed job.

For MCP `run`, validation occurs before creating a job. For MCP `wait`, validation occurs before observing the existing job and leaves that job untouched.

A configured value of zero is valid and provides launch/status-style immediate return behavior for omitted `until` calls.

## Rationale

One environment variable matches the actual integration decision: the MCP host knows the safe duration agent-exec may occupy a request. It avoids redundant CLI flags and avoids encoding OpenCode-specific timeout arithmetic in agent-exec. Using the same value as default and maximum prevents omitted and explicit calls from bypassing each other.

## Compatibility

An absent environment variable preserves the current MCP contract. Existing callers retain 10 seconds for omitted `run.until`, 30 seconds for omitted `wait.until`, and unrestricted explicit non-negative integer values.
