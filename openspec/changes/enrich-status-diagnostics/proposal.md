---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/status.rs
  - src/schema.rs
  - src/list.rs
  - src/delete.rs
  - src/jobstore.rs
  - src/serve.rs
  - tests/integration.rs
  - tests/support/mod.rs
  - schema/agent-exec.schema.json
  - CHANGELOG.md
  - README.md
  - skills/agent-exec/references/cli-contract.md
  - skills/agent-exec/references/completion-events.md
verifications:
  - id: status-contract-tests
    requirement: Status responses expose persisted execution context and live observability with bounded I/O, explicit presence semantics, and no sensitive input disclosure
    phase: pre-integration
    owner: conflux-acceptance
    trigger: change-implementation
    automation: tests/integration.rs
    evidence: cargo test --test integration status_
    rerun: cargo test --test integration status_
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Enrich status diagnostics

**Change Type**: implementation

## Problem / Context

`agent-exec status <job_id>` currently reports lifecycle state and timestamps, but omits execution context and observability data already available from `meta.json`, `state.json`, and canonical log files. A caller cannot tell what command is running, where it runs, whether the recorded PID appears alive, how long the current run has been observed, why it terminated, whether logs are fully drained, or whether either log contains output without additional calls or internal-file access.

`list` already reconciles stale persisted `running` state with a best-effort PID probe, while `status` returns persisted state. These surfaces need an explicit relationship rather than silently changing the existing `state` meaning.

## Proposed Solution

Extend successful status responses with backward-compatible diagnostic fields derived from existing job metadata, state, and log files:

- `command`, `cwd`, and `tags` from persisted metadata.
- `pid` and `process_alive` from persisted state plus the existing best-effort process probe.
- `updated_at`, live `elapsed_ms`, persisted terminal `duration_ms`, `signal`, and `logs_drained` from state.
- `stdout_log_path`, `stderr_log_path`, `stdout_total_bytes`, and `stderr_total_bytes` from canonical job log files.

Keep `state` as persisted lifecycle state. A stale running record is represented as `state="running"` plus `process_alive=false`; `status` remains read-only. Extract one tri-state process probe and reuse it in `list`, `delete`, and `status`, while preserving each existing caller's unsupported-platform fallback.

Obtain log sizes with file metadata only. `status` must not read log contents, so its memory and I/O cost remains independent of log size.

Advance the global stdout contract from schema `0.2` to `0.3`, update every tracked contract/documentation assertion, and preserve all existing field names, types, and meanings. The new fields remain optional at the published compatibility boundary even when the current implementation can always derive some of them.

## Acceptance Criteria

- `status` reports command, cwd, tags, PID, process liveness when observable, state update time, live elapsed duration, persisted terminal duration, terminating signal, log-drain state, canonical log paths, and current log byte totals according to the spec presence table.
- `elapsed_ms` is emitted only for non-terminal jobs with `started_at` and means response-time minus `started_at`; `duration_ms` is emitted only when persisted and retains its existing terminal-duration meaning.
- A running job reports `process_alive=true` while its PID appears live and `false` for an absent or dead PID; non-running jobs are never probed and omit the field.
- `state` retains its persisted meaning. `list` may present the same stale record as `unknown`; neither read command rewrites `state.json`.
- Log byte totals use `std::fs::metadata(...).len()` or equivalent bounded file-size queries. Missing or unreadable log files report zero bytes; log contents are never read by `status`.
- Status output never includes environment-variable values, stdin content, notification secrets, or shell-expanded command strings.
- The checked-in JSON Schema corrects the existing StatusResponse drift: `created_at` is defined, `started_at` is not required for created jobs, and `created` is an allowed state.
- The checked-in JSON Schema, changelog, source constant/comments, tests, README, and bundled skill references agree on schema `0.3`; all emitted envelopes use the global version.
- The `0.3` changelog states that completion and output-match event envelopes also carry `schema_version="0.3"` without field-shape changes because the version is global.
- CLI, HTTP serve, embedded, and MCP status surfaces continue using the same `StatusData` response type.

## Explicit Completion Conditions

- `src/status.rs` constructs the enriched response from canonical `JobDir`, `JobMeta`, and `JobState` data without reading log contents.
- `src/schema.rs` defines the new fields with explicit types and backward-compatible serialization.
- One shared liveness helper returns `Option<bool>` and is reused by `src/list.rs`, `src/delete.rs`, and `src/status.rs`. Existing unsupported-platform behavior remains unchanged for list and delete; status omits an unavailable observation.
- Integration tests execute the binary against isolated jobs and cover live PID, dead/absent PID, terminal PID omission, elapsed/duration separation, signal, logs-drained, log paths/sizes, missing logs, large logs, read-only state behavior, sensitive-input non-disclosure, and checked-in schema validation.
- Every new integration test name starts with `status_`; the evidence filter must execute at least the named tests rather than succeeding with zero matches.
- `schema/agent-exec.schema.json`, `CHANGELOG.md`, `src/schema.rs`, `tests/support/mod.rs`, `tests/integration.rs`, `README.md`, `skills/agent-exec/references/cli-contract.md`, and `skills/agent-exec/references/completion-events.md` agree on `0.3`.
- `cargo test --test integration status_` and `make check` pass.

## Security Note

`GET /status/:id` belongs to `serve`'s unauthenticated read-only route set. This change therefore makes command and cwd visible to peers already able to reach that route. This is not a new exposure class because `GET /tail/:id` already exposes log paths and contents on the same route, and non-loopback binds require the existing insecure-mode safeguards. Extending authentication to read-only routes is a separate change.

## Out of Scope

- Embedding log excerpts in `status`; callers continue using `tail` for content.
- Reusing `JobDir::read_tail_metrics` for status byte totals because that helper reads content.
- Persisting new job metadata or changing the job directory layout.
- Mutating stale `running` state during a read-only query.
- Authoritative process identity, child-process health, CPU/memory metrics, progress inference, or guarantees against PID reuse.
- Adding `timeout_ms`, remaining-time estimates, or kill-grace configuration to status.
- Exposing environment values, stdin content, notification secrets, or shell-expanded commands.
- Changing authentication policy for serve read-only routes.
- Repairing the pre-existing `JobMeta.schema_version` inconsistency between paths using `SCHEMA_VERSION` and legacy literals.

## Verification Ownership

Requirement-specific behavior is owned by `status-contract-tests`. Repository-wide fmt, clippy, and test checks remain final acceptance gates; `prek.toml` is staged-file scoped and is not treated as an unconditional clean-tree verification hook.
