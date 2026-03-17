# Change Proposal: add-create-start-lifecycle

## Problem/Context

`agent-exec` currently models job creation and execution as a single `run` action.

- `src/main.rs` exposes `run`, `status`, `tail`, `wait`, `kill`, `gc`, and `list`, but there is no way to create a durable job definition without starting it immediately.
- `src/run.rs` creates the job directory, writes `meta.json`, spawns `_supervise`, and initializes `state.json` as `running` in one flow.
- `src/schema.rs` only models terminal and active execution states (`running|exited|killed|failed`), so there is no persisted pre-start lifecycle state.
- `meta.json` currently preserves only `env_keys` plus masked display values, while `--env-file` is treated as a runtime input rather than durable job configuration.
- The user wants a two-step lifecycle: create a job first, then start it later, while keeping `--env` values persisted and keeping `--env-file` as persisted file-path references.

## Proposed Solution

Add a two-step job lifecycle with `create` and `start`, while keeping `run` as the compatibility path for immediate execution.

- Add `agent-exec create` to persist a full job definition without launching the supervisor or child process.
- Add `agent-exec start <job_id>` to launch a previously created job, reusing the persisted command, cwd, environment configuration, notification settings, timeout settings, and shell wrapper.
- Introduce a non-terminal `created` job state so `status`, `list`, `wait`, and `kill` can distinguish jobs that exist but have not started yet.
- Persist `--env KEY=VALUE` entries in `meta.json` as durable job configuration, and persist `--env-file <FILE>` as file-path references that are re-read at `start` time.
- Keep `run` available, but align its implementation and documentation with the new lifecycle so it behaves as the immediate-start convenience path.

## Acceptance Criteria

- `agent-exec create -- <cmd>` creates `<root>/<job_id>/` with `meta.json`, `state.json`, `stdout.log`, `stderr.log`, and `full.log`, returns `type="create"`, and leaves the job in `state="created"` without spawning the child process.
- `agent-exec start <job_id>` starts only jobs in `created` state, returns `type="start"`, and exposes the same snapshot / wait response behavior that `run` currently provides for immediate execution.
- `meta.json` persists enough execution configuration for `start` to run without re-specifying the original command-line definition, including `command`, `cwd`, `inherit_env`, `env_vars`, `env_files`, timeout-related settings, notification settings, masking keys, and resolved shell wrapper inputs.
- `--env` values are persisted as `KEY=VALUE` strings; `--env-file` values are persisted as file paths and are read when `start` constructs the runtime environment.
- `status`, `list`, `wait`, and `kill` handle `created` jobs deterministically: `list --state created` is supported, `wait` does not treat `created` as terminal, and `start` rejects jobs that have already been started or finished.
- Existing `run` behavior remains available as the immediate-start path, with integration coverage documenting the compatibility contract.

## Out of Scope

- Allowing the same job ID to be started multiple times after it has already entered `running`, `exited`, `killed`, or `failed`.
- Adding a secure secret store or changing the repository-wide masking mechanism beyond what is needed to document and preserve persisted non-secret env values.
- Changing the job root resolution order, log file layout, notification event format, or Windows process-tree rules outside the lifecycle additions required for `create` and `start`.
