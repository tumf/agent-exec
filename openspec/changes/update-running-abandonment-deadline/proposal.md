---
change_type: implementation
priority: high
dependencies:
  - rename-timeout-to-abandon-job-after
references:
  - src/main.rs
  - src/run.rs
  - src/status.rs
  - src/schema.rs
  - src/jobstore.rs
  - src/start.rs
  - src/restart.rs
  - src/mcp.rs
  - src/serve.rs
  - src/embedded.rs
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-mcp/spec.md
  - openspec/specs/agent-exec-serve/spec.md
  - openspec/changes/rename-timeout-to-abandon-job-after
  - tests/integration.rs
  - tests/mcp_integration.rs
  - tests/serve_integration.rs
verifications:
  - id: mutable-abandonment-tests
    requirement: Running-job abandonment deadlines can be safely changed or cleared and status reports the effective control without stale-trigger races
    phase: pre-integration
    owner: conflux-acceptance
    trigger: change-implementation
    automation: tests/integration.rs
    evidence: cargo test mutable_abandonment
    rerun: cargo test mutable_abandonment
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Update running-job abandonment deadlines

**Change Type**: implementation

## Premise / Context

- `rename-timeout-to-abandon-job-after` establishes the destructive control as `abandon-job-after` and separates it from non-destructive observation through `until`.
- A launch-time abandonment deadline may later prove too short, too long, or unnecessary.
- Editing persisted metadata alone is unsafe because the detached supervisor currently owns runtime timing and could still fire a stale deadline.
- Operators need the effective deadline visible in `status` without reading job-local files.

## Requested Artifact

- implementation

## Problem / Context

A running job's abandonment deadline is currently fixed at launch. An operator cannot safely extend, shorten, or clear it after observing progress. A metadata-only edit would not provide a linearizable contract with the supervisor and could falsely claim that a deadline was changed after abandonment had already begun.

## Proposed Solution

- Add `agent-exec abandon set <job_id> --in <seconds> --acknowledge-result-loss` for running jobs. `--in` is relative to the instant the update is durably accepted.
- Add `agent-exec abandon clear <job_id>` to remove a running job's active abandonment deadline. Clearing is non-destructive and requires no result-loss acknowledgement.
- Expose equivalent MCP, HTTP, embedded, and public Rust operations.
- Persist a revisioned job-local abandonment-control record used by both updater and supervisor.
- Serialize update, clear, and deadline-trigger transitions through the same job-local exclusive lock. An update that wins changes the effective deadline; a trigger that wins makes later changes fail with `invalid_state`.
- Resume the effective persisted deadline and revision after restart instead of restoring the original launch value.
- Extend `status` with the effective configured duration, absolute deadline, response-time remaining duration, revision, and configuration source.

## Acceptance Criteria

- A running active job's deadline can be shortened or extended from the update acceptance time.
- A running active job's deadline can be cleared without signaling the job.
- Every set operation requires explicit result-loss acknowledgement; rejected operations do not mutate control state.
- Created, terminal, or already-triggered jobs reject runtime deadline changes with stable job-domain errors.
- Update and trigger are linearizable: no successful update can be followed by a stale deadline signal, and no update claims success after trigger begins.
- Restart preserves the latest accepted absolute deadline and revision.
- CLI, MCP, HTTP, embedded, and public Rust operations share the same semantics and error behavior.
- `status` reports `abandon_job_after_ms`, `abandon_deadline`, `abandon_remaining_ms`, `abandon_revision`, and `abandon_configured_by` from the effective durable control.
- Disabled and terminal status values use documented null semantics; remaining time never becomes negative.

## Explicit Completion Conditions

- Real managed-job tests prove shorten, extend, and clear behavior.
- Deterministic synchronization tests prove both update-wins and trigger-wins race outcomes.
- Restart tests prove the latest revision survives and the launch-time deadline is not restored.
- CLI, MCP, and HTTP boundary tests prove acknowledgement and invalid-state handling before mutation.
- Status/schema tests prove field presence, null semantics, revision/source consistency, and response-time remaining calculation.
- `cargo test mutable_abandonment`, strict validation, archive-gate validation, `make check`, and `git diff --check` pass.

## Dependencies

This change depends on `rename-timeout-to-abandon-job-after` because it consumes the renamed public control, result-loss acknowledgement contract, persisted migration fields, and abandonment result markers after they are integrated into the base.

## Out of Scope

- Changing observation `until`.
- Editing deadlines after abandonment enters its triggered phase.
- Changing `kill-after` escalation timing.
- Scheduling multiple future deadline changes.
- Automatically extending deadlines based on output or activity.

## Verification Ownership

Requirement-specific behavior is owned by `mutable-abandonment-tests` across CLI, MCP, HTTP, persistence, race, restart, and status paths. Repository-wide checks remain final acceptance gates.
