# Design: distinguish observation from workload termination

## Naming boundary

- `until`: caller observation duration. Expiry returns control and never signals the job.
- `max-runtime` / `max_runtime`: managed workload lifetime ceiling. Expiry sends `SIGTERM` and may escalate to `SIGKILL` after `kill-after`.

`runtime-timeout` was considered but rejected because it retains the overloaded word that caused the incident. `max-runtime` names the bounded resource directly and avoids implying a client/request timeout.

## Compatibility

Public aliases for `timeout` are not retained. A warning-only deprecation period would leave the unsafe name selectable by agents and continue the exact failure mode.

Persisted data differs: old jobs must remain restartable. Deserialization accepts legacy `timeout_ms` as an alias only for stored metadata. Serialization emits `max_runtime_ms`. If both fields exist, loading fails closed rather than choosing one silently.

The terminal state string `timeout` remains unchanged. Renaming it would break historical job responses without reducing launch-time ambiguity.

## Surface mapping

| Surface | Old | New |
|---|---|---|
| CLI | `--timeout` | `--max-runtime` |
| MCP run | `timeout` | `max_runtime` |
| HTTP `/exec` | `timeout` | `max_runtime` |
| Embedded request | `timeout_ms` | `max_runtime_ms` |
| Persisted metadata | `timeout_ms` | write `max_runtime_ms`; read legacy `timeout_ms` |
| Observation | `until` | unchanged |

## Verification strategy

Use one focused integration-test filter that exercises the compiled binary and request boundaries. Include a short process that exceeds `max-runtime`, a longer process observed with `until`, and metadata fixtures for legacy/new/conflicting fields. This proves behavior rather than only checking help or spec text.
