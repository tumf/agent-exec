---
change_type: implementation
priority: high
dependencies: []
references:
  - src/run.rs
  - src/start.rs
  - src/restart.rs
  - src/serve.rs
  - tests/integration.rs
  - tests/mcp_integration.rs
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-mcp/spec.md
verifications:
  - id: supervisor-reaping-tests
    requirement: Unix supervisor children are reaped by long-lived launchers without changing detached managed-job behavior
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: tests/integration.rs
    evidence: cargo test supervisor_reaping
    rerun: cargo test supervisor_reaping
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
  - id: mcp-lifecycle-tests
    requirement: Repeated short jobs through one MCP server leave no exited supervisor children and client disconnect still does not cancel jobs
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: tests/mcp_integration.rs
    evidence: cargo test --test mcp_integration mcp_reaps_finished_supervisors
    rerun: cargo test --test mcp_integration mcp_reaps_finished_supervisors
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
  - id: rust-quality-gates
    requirement: Rust formatting, lint, and regression suite remain clean
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: prek run -a output
    rerun: prek run -a
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Reap detached supervisor processes

**Change Type**: implementation

## Problem / Context

`spawn_supervisor_process` starts the detached agent-exec supervisor with `Command::spawn()` and immediately drops the returned `Child` after reading its PID. On Unix, dropping `Child` does not wait for the process. A long-lived caller such as `agent-exec mcp` therefore retains every finished supervisor as a zombie child until the MCP process exits.

The issue is observable on the mini host: 2,155 zombie processes were present, with the majority parented by long-lived `agent-exec mcp` instances and one instance retaining 961 zombies. The per-user process limit was 2,666, so continued accumulation can prevent new processes from starting.

The spawn path is shared by `run`, `start`, `restart`, HTTP serve, and MCP. The fix must therefore live at the common supervisor launch boundary rather than in MCP-specific request handling. Managed jobs must remain detached: caller shutdown, MCP disconnect, and observation deadlines must not signal or synchronously wait for workload completion.

## Proposed Solution

On Unix, retain ownership of each spawned supervisor `Child` in a non-blocking reaping path that calls `wait()` after the supervisor exits. The caller must still receive the supervisor PID and initial state immediately, and the reaping path must not couple supervisor lifetime to the CLI, MCP transport, or HTTP request lifetime.

Use per-child ownership rather than installing a process-wide `SIGCHLD` handler or calling broad `waitpid(-1, ...)`. This ensures agent-exec only reaps supervisors it spawned and cannot consume child status owned by another library. Preserve the existing Windows Job Object handshake and process lifecycle behavior.

This is one atomic change because the common spawn ownership fix and the Unix/MCP regressions are required together to prove both resource cleanup and detachment compatibility.

## Acceptance Criteria

- A long-lived `agent-exec mcp` process can execute repeated short jobs without accumulating exited supervisor children in zombie state.
- `run`, `start`, `restart`, HTTP serve, and MCP continue using the common supervisor spawn path, so no execution surface bypasses supervisor reaping.
- Supervisor launch remains non-blocking; `run` and `start` retain their existing bounded inline-observation behavior rather than waiting for workload completion.
- Closing an MCP client or terminating a short-lived launcher does not cancel a managed job or change its persisted state contract.
- Reaping is scoped to each `Child` returned by agent-exec's own `Command::spawn()`; no process-wide signal handler or indiscriminate child wait is introduced.
- Windows behavior, including the existing Job Object handshake, remains unchanged.
- No CLI flags, JSON fields, job metadata, state schema, log format, or exit-code contract changes.
- Unix regression coverage fails against the current implementation by detecting unreaped finished supervisor children and passes after the fix.
- Any test requiring more than one second is marked `heavy`; the default regression suite remains under the repository's unit-test duration policy.

## Explicit Completion Conditions

- `src/run.rs` transfers each Unix supervisor `Child` to an owned reaping mechanism while still returning the PID and `started_at` without waiting for supervisor completion.
- Existing callers in `src/run.rs`, `src/start.rs`, `src/restart.rs`, and `src/serve.rs` remain wired through `spawn_supervisor_process` and require no surface-specific cleanup workaround.
- A Unix process-level regression executes short managed jobs from a launcher that remains alive, waits only within a bounded sub-second test deadline, and proves no finished supervisor remains as a zombie child.
- `tests/mcp_integration.rs` proves repeated short MCP jobs are reaped by the still-running MCP server and preserves the existing disconnect-does-not-cancel contract.
- `cargo test supervisor_reaping` passes.
- `cargo test --test mcp_integration mcp_reaps_finished_supervisors` passes.
- `prek run -a` passes without weakening hooks, warnings, or existing tests.

## Out of Scope

- Killing or restarting currently running OpenCode, Hermes, MCP, or HTTP server processes to clear already accumulated zombies.
- Cleaning up zombies created by unrelated programs, including the observed Conflux test process.
- Changing managed-job detachment, process-group signaling, timeout, kill-after, or output-drain semantics.
- Adding a global child-process manager, async runtime dependency, daemon, CLI option, or persisted schema.
- Changing Windows process ownership or Job Object behavior beyond compiling and passing existing Windows-compatible checks.
