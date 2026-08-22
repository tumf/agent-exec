# Design: distinguish observation from workload termination

## Naming boundary

- `until`: caller observation duration. Expiry returns control and never signals the job.
- `abandon-job-after` / `abandon_job_after`: an explicit decision to give up on the job. Expiry sends `SIGTERM`, may escalate to `SIGKILL`, and can permanently lose unfinished results.

`runtime-timeout`, `max-runtime`, and `terminate-job-after` were rejected. They sound like ordinary scheduling controls and understate result loss. `abandon-job-after` states that the caller is giving up on completion rather than merely ending observation.

## Warning and acknowledgement

Naming is the first warning, not the only warning. CLI help and structured API field descriptions begin with: `WARNING: Gives up on the job, terminates it, and may permanently lose unfinished results. Use until to stop waiting without stopping the job.`

The destructive control requires a separate acknowledgement at each public boundary: CLI `--acknowledge-result-loss`, MCP/HTTP `acknowledge_result_loss=true`, and an equivalent explicit embedded request boolean. Missing acknowledgement fails before persistence or launch. Persisted legacy jobs remain restartable without retroactive acknowledgement because they were already created under the old contract.

## Compatibility

Public aliases for `timeout` are not retained. A warning-only deprecation period would leave the unsafe name selectable by agents and continue the exact failure mode.

Persisted data differs: old jobs must remain restartable. Deserialization accepts legacy `timeout_ms` as an alias only for stored metadata. Serialization emits `abandon_job_after_ms`. If both fields exist, loading fails closed rather than choosing one silently.

The terminal state string `timeout` remains unchanged. Renaming it would break historical job responses without reducing launch-time ambiguity.

## Surface mapping

| Surface | Old | New |
|---|---|---|
| CLI | `--timeout` | `--abandon-job-after` |
| MCP run | `timeout` | `abandon_job_after` |
| HTTP `/exec` | `timeout` | `abandon_job_after` |
| Embedded request | `timeout_ms` | `abandon_job_after_ms` |
| Persisted metadata | `timeout_ms` | write `abandon_job_after_ms`; read legacy `timeout_ms` |
| Observation | `until` | unchanged |

## Verification strategy

Use one focused integration-test filter that exercises the compiled binary and request boundaries. Include missing/false acknowledgement rejection, a short acknowledged process that exceeds `abandon-job-after`, a longer process observed with `until`, warning text assertions, and metadata fixtures for legacy/new/conflicting fields. This proves behavior rather than only checking help or spec text.
