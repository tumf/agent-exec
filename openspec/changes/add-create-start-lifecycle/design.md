# Design: add-create-start-lifecycle

## Summary

This change introduces an explicit two-step lifecycle for jobs:

1. `create` persists a job definition and writes a `created` state.
2. `start` consumes that persisted definition and begins execution.

`run` remains supported as the convenience path for immediate execution, but its implementation should be aligned with the same lifecycle primitives so job creation and job start are no longer inseparable.

## Current State

- `src/run.rs` generates a job ID, writes `meta.json`, pre-creates log files, spawns `_supervise`, and initializes `state.json` as `running` in one function.
- `src/schema.rs` models `JobStatus` as `running|exited|killed|failed`, so there is no persisted non-started state.
- `src/status.rs` and `src/list.rs` effectively treat `meta.created_at` as the only user-visible start timestamp.
- `src/wait.rs` considers every non-`running` state terminal, which would incorrectly cause a newly introduced pre-start state to return immediately.
- `meta.json` currently stores only `env_keys` plus masked display-oriented `env_vars`, which is not sufficient to reconstruct the original runtime configuration for a later `start`.

## Lifecycle Model

### States

The job lifecycle becomes:

- `created`: job definition exists but no process has been launched yet
- `running`: supervisor/child process is active
- `exited`, `killed`, `failed`: existing terminal states

### Allowed Transitions

- `create` -> `created`
- `start(created)` -> `running`
- `running` -> `exited|killed|failed`

`start` must reject any job that is not currently `created`. This keeps one job ID mapped to one actual execution attempt and avoids ambiguous re-run semantics for logs, completion events, and state history.

## Persisted Metadata

`meta.json` must carry the complete startable definition for a created job.

Recommended durable fields:

- `command`
- `created_at`
- `cwd`
- `inherit_env`
- `env_vars` as persisted `KEY=VALUE` strings from `--env`
- `env_files` as persisted file paths from `--env-file`
- `mask`
- `timeout_ms`
- `kill_after_ms`
- `progress_every_ms`
- `notification`
- `shell_wrapper`
- existing root / schema metadata

### Env Persistence Rules

- `--env KEY=VALUE` is treated as durable non-secret configuration and is stored in `meta.json`.
- `--env-file FILE` remains a file-path reference. `create` stores the path; `start` reads the file contents when building the runtime environment.
- Environment construction order remains unchanged from the current `run` semantics: inherited environment (unless disabled), then `env_files` in CLI order, then `env_vars` overriding later.
- `--mask` continues to control display in JSON and persisted display fields; it does not alter the real process environment.

## State Timestamps

The existing state shape assumes `job.started_at` is always available. A `created` state breaks that assumption.

The proposal should separate creation time from execution start time:

- `meta.created_at` remains the durable creation timestamp.
- `state.job.started_at` should become nullable until `start` actually launches the job.
- User-facing payloads such as `status` should expose both `created_at` and `started_at` where needed, or otherwise document that `started_at` is absent for created jobs.

This is preferable to overloading `started_at` with creation time because the new lifecycle makes those moments observably different.

## Command Behavior

### `create`

`create` accepts definition-time options only. It should:

- allocate `job_id`
- write `meta.json`
- pre-create `stdout.log`, `stderr.log`, and `full.log`
- write `state.json` as `created`
- return JSON with `type="create"`

`create` must not spawn `_supervise` or a child process.

### `start`

`start` accepts observation-time options (`snapshot_after`, tail limits, wait flags) and should:

- load the persisted metadata for `job_id`
- reject jobs not in `created`
- spawn `_supervise` using the persisted execution definition
- transition state from `created` to `running`
- return a JSON payload parallel to the existing `run` response shape, but with `type="start"`

### `run`

`run` should remain the immediate execution entrypoint for compatibility. Internally it should reuse the same creation and start helpers so the lifecycle rules are defined in one place.

## Consumer Semantics

### `status`

`status` must distinguish `created` from `running` and should no longer imply that every job has already started executing.

### `wait`

`wait` must treat `created` as non-terminal. A caller waiting on a created job should continue polling until the job reaches `running` and eventually a terminal state, or until the wait timeout expires.

### `kill`

`kill` should reject `created` jobs because there is no running process tree to signal. Returning success for a never-started job would hide invalid lifecycle usage.

### `list`

`list --state` must add `created` to the accepted state set, and list summaries must expose `created` deterministically for not-yet-started jobs.

## Verification Impact

Integration coverage should prove:

1. `create` writes durable job data without executing the command.
2. `start` reconstructs the runtime environment from persisted `env_vars` and `env_files`.
3. `run` still behaves as the immediate-start path.
4. `wait`, `kill`, `status`, and `list` implement the new lifecycle rules consistently.
5. JSON output remains one-object-only on stdout despite the expanded lifecycle.
