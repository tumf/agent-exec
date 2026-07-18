## Context

The current MCP startup policy stores only `max_until_seconds`. `until_seconds` uses that value as the omitted default and rejects larger explicit values. The follow-up contract needs independent default selection and a non-failing safety cap.

## Decision

Parse two optional non-negative integer environment variables once before serving:

- `AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS`
- `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`

Resolve each tool call in two stages:

1. Select the requested value using `explicit -> configured default -> tool legacy default`.
2. If a maximum exists, return `min(requested, maximum)`; otherwise return the requested value.

The resulting value is passed to the existing canonical `run` or `wait` path.

## Precedence Matrix

| Explicit | Default env | Max env | Effective |
| --- | --- | --- | --- |
| 20 | 55 | 60 | 20 |
| 100 | 55 | 60 | 60 |
| omitted | 55 | 60 | 55 |
| omitted | 100 | 60 | 60 |
| omitted | absent | 5 | 5 for both tools |
| omitted | absent | absent | 10 for run, 30 for wait |
| 100 | absent | absent | 100 |

Zero is valid for either variable. A zero maximum forces immediate-return observation semantics for both explicit and omitted calls.

## Error Behavior

Malformed startup environment values fail before stdio protocol serving. The error identifies whether the default or maximum variable is invalid.

Valid over-cap tool requests do not produce an error. They proceed with the capped value. Input-shape errors such as negative, fractional, non-finite, or out-of-range MCP `until` values remain protocol-safe tool errors before clamping.

## Managed-Job Semantics

Clamping changes only the observation duration. MCP `run` still creates and launches a detached managed job. MCP `wait` still leaves the existing job untouched when the effective deadline expires. No cap event signals or cancels a job.

## Compatibility

When both variables are absent, behavior is identical to pre-configuration MCP behavior: omitted run uses 10 seconds, omitted wait uses 30 seconds, and valid explicit values are uncapped.

Existing hosts that set only `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` will change in two intentional ways: omission returns to legacy per-tool defaults subject to the cap, and over-cap explicit calls are clamped instead of rejected.
