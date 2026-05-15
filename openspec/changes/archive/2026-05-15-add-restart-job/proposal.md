---
change_type: implementation
priority: high
dependencies: []
references:
  - src/main.rs
  - src/start.rs
  - src/run.rs
  - src/kill.rs
  - src/jobstore.rs
  - src/schema.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-jobstore/spec.md
---

# Add restart command for existing jobs

**Change Type**: implementation

## Problem / Context

`agent-exec` can start newly created jobs and kill running jobs, but it does not provide a single command that replaces an existing job process while preserving the same `job_id` and job directory. Users who want to retry or refresh a job currently need to create a new job, which changes references used by agents, terminals, logs, and status/tail commands.

The repository already has the core primitives needed for this behavior:

- `start` reads persisted `meta.json` and launches a job through `spawn_supervisor_process`.
- `kill` can terminate a running job process tree.
- `spawn_supervisor_process` accepts an existing `JobDir` and writes a fresh `running` state for the same `job_id`.
- `run`/`start` share the inline observation response shape via `RunData`.

## Proposed Solution

Add a new `agent-exec restart <job_id>` command that preserves the target `job_id` and job directory while replacing the active process with a fresh launch from the persisted job definition.

The command should:

- Accept any existing job state (`created`, `running`, `exited`, `killed`, `failed`) that has a usable persisted definition.
- For `running` jobs, send a configurable termination signal to the current process tree and wait for termination before launching the replacement process.
- For terminal jobs, launch a new process from `meta.json` without creating a new job directory.
- For `created` jobs, behave like `start` so users can use `restart` as an idempotent "ensure this job definition is running now" command.
- Reset per-run log/output artifacts before the fresh launch so `tail`, inline observation, and completion records reflect the current run rather than mixed historical output.
- Return JSON-only stdout with `type="restart"` and the same top-level observation fields as `run`/`start`.

## Acceptance Criteria

- `agent-exec restart <job_id>` returns a successful JSON response with the same `job_id` and `type="restart"` when restarting a restartable job.
- Restarting a `running` job terminates the previous process tree before launching the replacement process.
- Restarting a terminal job reuses the persisted command definition and does not create a new job directory or new `job_id`.
- Restarting a `created` job launches it using the same semantics as `start`.
- Restart responses support the same inline observation controls as `start`: `--wait`, `--until`, `--forever`, `--no-wait`, and `--max-bytes`.
- Restart preserves `meta.json` identity and persisted definition fields, including command, cwd, env-file references, durable env values for `create` jobs, stdin file reference, tags, notifications, runtime limits, and shell wrapper.
- Restart resets per-run output artifacts before relaunch so `stdout.log`, `stderr.log`, and `full.log` represent the new run's output.
- Restart does not write any non-JSON text to stdout and keeps stable error handling for missing or invalid job ids.

## Explicit Completion Conditions

This proposal is complete when repository evidence shows:

- `src/main.rs` exposes a `Restart` CLI variant with completions and observation flags consistent with `start`.
- A restart implementation module or equivalent runtime path reuses existing job metadata and supervisor launch logic rather than generating a new `job_id`.
- Running-job restart performs real termination and waits for a terminal state before relaunching, with Windows process-tree handling preserved through the existing kill pathway or an equivalent shared helper.
- Per-run output artifacts are cleared before the new supervisor writes output.
- `tests/integration.rs` covers running, terminal, and created restart paths with assertions that would fail for a no-op or dummy response.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.
- `cflx openspec validate add-restart-job --strict` passes.

## Out of Scope

- Creating a multi-run history model inside a single job directory.
- Adding restart counters or run attempt IDs to the public schema.
- Changing `run` job id generation behavior.
- Changing `start` so it can directly start terminal jobs.
- Preserving old per-run logs in rotated files.
