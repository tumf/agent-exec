# Design: Restart existing jobs without changing job id

## Overview

`restart` is a lifecycle command that reuses an existing job directory and persisted definition while replacing the process currently associated with that job. It should feel like `start` for persisted definitions and like `kill` + `start` for running jobs, but it must be atomic enough that agents can keep referencing the same `job_id`.

## Command Shape

```bash
agent-exec restart [--signal TERM|INT|KILL|...] [--wait true|false] [--until SECONDS | --forever] [--no-wait] [--max-bytes BYTES] <job_id>
```

The response uses the existing `RunData` fields with envelope `type="restart"`.

## State Handling

| Current state | Restart behavior |
| --- | --- |
| `created` | Launch persisted definition, equivalent to `start`, but response type is `restart`. |
| `running` | Terminate current process tree, wait for non-running observation, reset per-run artifacts, then launch persisted definition. |
| `exited` / `killed` / `failed` | Reset per-run artifacts and launch persisted definition. |

The command should reject only jobs that cannot be opened or whose persisted definition is unusable.

## Process Termination

The implementation should prefer reusing existing kill logic so Unix process-group behavior and Windows Job Object behavior remain aligned with `kill`.

A safe implementation path is:

1. Read current `state.json`.
2. If status is `running`, send the selected signal using shared kill functionality.
3. Observe state until terminal or until a bounded termination budget expires.
4. If graceful termination fails and the selected signal was not `KILL`, escalate to `KILL` or return a retryable/error response rather than launching a second process under the same job id.
5. Only after the old process is no longer running, reset per-run artifacts and launch the new supervisor.

## Relaunch Semantics

Relaunch should follow `start` metadata semantics:

- command from `meta.command`
- cwd from `meta.cwd`
- env runtime values from `meta.env_vars_runtime` for `create` jobs
- env-file paths from `meta.env_files`, read at restart time
- inherit-env from `meta.inherit_env`
- stdin file from `meta.stdin_file`
- timeout, kill-after, progress-every from persisted metadata
- notification sinks from current metadata
- shell wrapper from persisted `meta.shell_wrapper` or config default fallback
- tags from persisted metadata for the JSON response

For jobs originally created by `run`, `env_vars_runtime` may be empty by design. Restart therefore reuses persisted env-file references and inherit-env behavior, but cannot recover ad-hoc unpersisted `run --env KEY=VALUE` runtime values beyond what is already stored in metadata. The implementation should preserve the current metadata contract and avoid storing new secrets as part of this change.

## Log and Artifact Reset

Restart should make the new run easy for agents to observe. Before relaunch, truncate:

- `stdout.log`
- `stderr.log`
- `full.log`

Remove stale completion artifacts when present:

- `completion_event.json`

Do not delete `meta.json`, `state.json`, `stdin.bin`, or user definition metadata. `notification_events.ndjson` may remain as event history unless the implementation chooses to document and test truncation; the required reset surface is the per-run logs and completion snapshot.

## Auto-GC

Successful restart should run bounded auto-GC with the same best-effort behavior as `run`/`start`. The target job must remain inspectable even if it is terminal before restart or becomes terminal during inline observation.

## Risks and Mitigations

- **Two live processes under one job id**: Do not relaunch until termination observation confirms the old process is no longer running.
- **Secret persistence regression**: Reuse existing persisted metadata; do not introduce new storage of real `run --env` values.
- **Mixed logs after restart**: Truncate per-run logs before relaunch and cover with integration tests.
- **Platform-specific process handling**: Share kill primitives rather than duplicating Unix/Windows process tree logic.
