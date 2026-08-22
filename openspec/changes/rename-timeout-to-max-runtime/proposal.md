---
change_type: implementation
priority: high
dependencies: []
references:
  - src/main.rs
  - src/mcp.rs
  - src/serve.rs
  - src/embedded.rs
  - src/schema.rs
  - README.md
  - skills/agent-exec/SKILL.md
  - skills/agent-exec/references/cli-contract.md
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-mcp/spec.md
  - openspec/specs/agent-exec-serve/spec.md
  - tests/integration.rs
verifications:
  - id: max-runtime-contract-tests
    requirement: All public launch surfaces name the destructive workload lifetime limit max-runtime and reject the ambiguous timeout spelling while preserving detached observation semantics and persisted-job compatibility
    phase: pre-integration
    owner: conflux-acceptance
    trigger: change-implementation
    automation: tests/integration.rs
    evidence: cargo test --test integration max_runtime
    rerun: cargo test --test integration max_runtime
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Rename timeout to max-runtime

**Change Type**: implementation

## Premise / Context

- `agent-exec` separates bounded observation (`until`) from managed workload lifetime control.
- The current public name `timeout` was mistaken for an observation deadline and used to stop a detached workload after three hours.
- `--timeout` is destructive: reaching it sends `SIGTERM`, then `SIGKILL` after `--kill-after`; `--until` never stops the job.
- The ambiguous name exists in CLI, MCP, HTTP `/exec`, embedded Rust API, documentation, skills, tests, and persisted metadata plumbing.
- Existing persisted jobs must remain restartable; compatibility for stored metadata is distinct from continuing to accept the unsafe public spelling.

## Requested Artifact

- implementation

## Problem / Context

The name `timeout` hides which lifetime it limits. In `agent-exec`, observation calls may return while a detached job continues, whereas the launch-time workload limit terminates the process tree and may prevent execution成果 from being produced. The two concepts must not share generic deadline terminology.

## Proposed Solution

- Rename the public destructive workload limit to `max-runtime` in CLI and `max_runtime` in JSON/MCP/Rust fields.
- Reject new CLI/MCP/HTTP requests using `timeout` or `timeout_ms`; do not retain a deprecated alias that agents can continue selecting accidentally.
- Keep `until` as the only bounded observation name and state explicitly that expiry never signals the job.
- Rename public help, schemas, examples, documentation, bundled skills, and tests consistently.
- Preserve restart/read compatibility for existing persisted `meta.json.timeout_ms` data. New persisted data uses `max_runtime_ms`; readers accept the legacy field only as stored-data migration input and reject records containing both names.
- Preserve the existing terminal state value `timeout` for backward-compatible historical result interpretation. This proposal renames the control input, not existing terminal-state JSON values.
- Keep `kill-after` behavior unchanged; it remains the escalation delay after `max-runtime` sends `SIGTERM`.

## Acceptance Criteria

- `agent-exec run --max-runtime 30 -- <cmd>` and `agent-exec create --max-runtime 30 -- <cmd>` apply a 30-second workload lifetime limit.
- Public `--help`, MCP schema, HTTP `/exec`, and embedded Rust API describe the control as `max-runtime` / `max_runtime`, explicitly stating that it terminates the workload.
- `--timeout`, MCP `timeout`, HTTP `timeout`, and public Rust request field `timeout_ms` are absent from current examples and rejected at public request boundaries.
- `until` expiry still returns a non-terminal observation without signaling or mutating the managed job.
- Existing persisted jobs containing only `timeout_ms` remain readable, startable, and restartable with identical runtime-limit behavior.
- Newly created metadata writes only `max_runtime_ms`; conflicting persisted legacy/new fields fail closed with a clear error.
- Runtime-limit termination behavior and terminal state serialization remain backward compatible.

## Explicit Completion Conditions

- CLI, MCP, HTTP, embedded API, public schema/docs/skills, and test fixtures use the new name.
- Integration tests execute real short-lived jobs proving `max-runtime` terminates a process and `until` does not.
- Boundary tests prove every old public spelling is rejected before a job is created.
- Persistence tests prove legacy metadata can restart and new metadata does not write `timeout_ms`.
- `cargo test --test integration max_runtime` and `make check` pass.

## Out of Scope

- Removing or renaming the existing terminal state value `timeout`.
- Changing `kill-after` timing or signal escalation behavior.
- Adding a default runtime limit; the default remains unlimited.
- Rewriting archived changes or historical job metadata in place.

## Verification Ownership

Requirement-specific behavior is owned by `max-runtime-contract-tests`. Repository-wide formatting, lint, and test checks remain final acceptance gates; staged-file hooks are not treated as clean-tree verification.
