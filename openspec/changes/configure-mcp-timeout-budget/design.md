## Context

`agent-exec mcp` synchronously serves tool calls while managed jobs execute independently under detached supervision. The MCP client, not the server, controls the outer JSON-RPC request deadline. A tool observation period longer than that deadline loses the response even though the job continues normally.

## Decision

Represent the integration constraint as one server-level timeout policy constructed at MCP startup:

- `client_timeout_seconds`: optional outer request deadline supplied by the platform integration.
- `safety_margin_seconds`: reserved response/transport headroom.
- `default_run_until_seconds`: used when MCP `run.until` is omitted.
- `default_wait_until_seconds`: used when MCP `wait.until` is omitted.
- `max_until_seconds`: when a client timeout is supplied, checked subtraction of safety margin from client timeout.

The policy is validated once before serving and reused by both tools. Per-call explicit `until` values are validated against `max_until_seconds` before invoking canonical run/wait behavior.

## Precedence

1. A per-call `until` value overrides the tool-specific default.
2. The selected value must fit within the configured client-safe maximum when one exists.
3. Without MCP timeout-policy options, legacy defaults remain in effect and no new maximum is imposed.

## Error Behavior

Invalid startup policy fails before stdio protocol serving begins. A per-call value above the maximum returns a protocol-safe tool error containing the requested value and permitted maximum. It does not start waiting, cancel the MCP server, or signal the managed job.

For MCP `run`, validation occurs before creating a job so an invalid observation request has no side effect. For MCP `wait`, validation occurs before observing the existing job and leaves that job untouched.

## Rationale

A single client timeout plus safety margin is less error-prone than independently configured undocumented maxima. Separate run and wait defaults remain necessary because the tools have different existing defaults and usage patterns. Keeping timeout configuration explicit avoids incorrectly hard-coding OpenCode's 60-second deadline for Hermes or future clients.

## Compatibility

No-option startup preserves the current MCP contract. Existing callers that omit `until` retain 10 seconds for `run` and 30 seconds for `wait`; explicit values remain accepted as before unless an integration opts into a client timeout budget.
