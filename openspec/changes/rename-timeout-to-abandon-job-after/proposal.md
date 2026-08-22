---
change_type: implementation
priority: high
dependencies: []
references:
  - src/main.rs
  - src/mcp.rs
  - src/serve.rs
  - src/embedded.rs
  - src/run.rs
  - src/create.rs
  - src/start.rs
  - src/restart.rs
  - src/schema.rs
  - README.md
  - CHANGELOG.md
  - skills/agent-exec/SKILL.md
  - skills/agent-exec/references/cli-contract.md
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-mcp/spec.md
  - openspec/specs/agent-exec-serve/spec.md
  - tests/integration.rs
  - tests/mcp_integration.rs
  - tests/serve_integration.rs
verifications:
  - id: abandon-job-contract-tests
    requirement: All public launch surfaces name the destructive control abandon-job-after, warn that it gives up on the job and can lose unfinished results, reject the ambiguous timeout spelling with migration guidance, preserve detached observation semantics, mark abandoned results, and retain persisted-job compatibility across one downgrade window
    phase: pre-integration
    owner: conflux-acceptance
    trigger: change-implementation
    automation: tests/integration.rs
    evidence: cargo test abandon_job_after
    rerun: cargo test abandon_job_after
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Rename timeout to abandon-job-after

**Change Type**: implementation

## Premise / Context

- `agent-exec` separates bounded observation (`until`) from managed workload lifetime control.
- The current public name `timeout` was mistaken for an observation deadline and used to stop a detached workload after three hours.
- `--timeout` is destructive: reaching it sends `SIGTERM`, then `SIGKILL` after `--kill-after`; `--until` never stops the job.
- The ambiguous name exists in CLI, MCP, HTTP `/exec`, public Rust APIs, documentation, skills, tests, and persisted metadata plumbing.
- Existing persisted jobs must remain restartable, and jobs created during migration must remain safe if an older binary starts or restarts them.

## Requested Artifact

- implementation

## Problem / Context

The name `timeout` hides which lifetime it limits. In `agent-exec`, observation calls may return while a detached job continues, whereas this launch-time control gives up on the job, terminates its process tree, and can permanently lose unfinished results. The two concepts must not share generic deadline terminology.

## Proposed Solution

- Rename the public destructive workload limit to `abandon-job-after` in CLI and `abandon_job_after` in JSON/MCP/public Rust fields.
- Make every public description start with an explicit warning: this option gives up on the job, terminates it, and may permanently lose unfinished results; use `until` when the intent is only to stop waiting.
- Require CLI use to include `--acknowledge-result-loss`. Supplying `--abandon-job-after` without that acknowledgement fails before job creation. Structured MCP/HTTP requests require `acknowledge_result_loss=true`; public Rust launch requests require the equivalent explicit boolean.
- Keep hidden CLI `--timeout` only as an always-failing migration trap. It must never launch a job and must identify both safe alternatives: `--abandon-job-after ... --acknowledge-result-loss` for intentional abandonment and `--until` for bounded observation. MCP/HTTP legacy fields receive equivalent protocol-safe migration errors before job creation.
- Keep `until` as the only bounded observation name and state explicitly that expiry never signals the job.
- Rename public help, schemas, examples, documentation, bundled skills, public Rust APIs, and tests consistently. Keep the private `_supervise --timeout` wire spelling unchanged so it cannot become a second public migration surface.
- Preserve restart/read and one-release downgrade compatibility for persisted metadata. During one migration release, writers dual-write equal `abandon_job_after_ms` and `timeout_ms`; readers accept either or both when equal and fail closed when both differ. The following release may stop writing `timeout_ms` after an explicit migration change.
- Preserve terminal `state="timeout"` for backward compatibility, but add `abandoned_by="abandon_job_after"` and `result_loss=true` to state/status/list/completion output when the configured abandonment actually terminates a workload. Schemas and guidance explain the legacy state mapping.
- Keep `kill-after` behavior unchanged; it remains the escalation delay after `abandon-job-after` sends `SIGTERM`.

## Acceptance Criteria

- `agent-exec run --abandon-job-after 30 --acknowledge-result-loss -- <cmd>` and the equivalent `create` command give up on and terminate the job after 30 seconds.
- Public `--help`, MCP schema, HTTP `/exec`, and public Rust APIs describe the control as `abandon-job-after` / `abandon_job_after`; the first sentence warns that it gives up on the job and may permanently lose unfinished results.
- Omitted or false result-loss acknowledgement rejects the request before job creation on every public launch surface.
- Legacy CLI/MCP/HTTP inputs are rejected before job creation with an error naming both `abandon-job-after` and non-destructive `until`; no deprecated legacy input launches a job.
- `until` expiry still returns a non-terminal observation without signaling or mutating the managed job.
- Existing persisted jobs containing only `timeout_ms` remain readable, startable, and restartable with identical runtime-limit behavior.
- Migration-release metadata dual-writes equal old/new fields so an older binary preserves the limit. Readers reject unequal dual fields with a job-specific error.
- Actual abandonment preserves `state="timeout"` and also emits `abandoned_by="abandon_job_after"` and `result_loss=true` through persisted state, status, list, and completion events.
- Public Rust `RunOpts`, `SuperviseOpts`, `CreateOpts`, and embedded request fields use the new name. The private supervisor parser/handoff remains internally consistent.
- Current README, changelog, site/docs, bundled skills, examples, schemas, and fixtures contain no live public `timeout` launch example except explicit migration/rejection documentation.

## Explicit Completion Conditions

- CLI, MCP, HTTP, public Rust APIs, public schema/docs/skills, and test fixtures use the new name.
- Integration tests execute real short-lived jobs proving `abandon-job-after` terminates a process and `until` does not.
- Boundary tests prove every old public spelling is rejected with migration guidance before a job is created.
- Boundary tests prove `abandon-job-after` is rejected without explicit result-loss acknowledgement and accepted with it.
- Persistence tests prove legacy-only metadata, equal dual-written metadata, unequal dual-field rejection, start/restart behavior, and old-binary downgrade preservation.
- Result tests prove abandonment markers appear only when the workload was actually abandoned.
- `cargo test abandon_job_after`, strict validation, archive-gate validation, `make check`, and `git diff --check` pass.

## Out of Scope

- Removing or renaming the existing terminal state value `timeout`.
- Renaming the private `_supervise --timeout` process handoff.
- Changing `kill-after` timing or signal escalation behavior.
- Adding a default runtime limit; the default remains unlimited.
- Rewriting archived changes or historical job metadata in place.
- Removing the legacy persisted `timeout_ms` write in the same release; that requires a later explicit migration change.

## Verification Ownership

Requirement-specific behavior is owned by `abandon-job-contract-tests` across CLI, MCP, and HTTP integration targets. Repository-wide formatting, lint, and test checks remain final acceptance gates; staged-file hooks are not treated as clean-tree verification.
