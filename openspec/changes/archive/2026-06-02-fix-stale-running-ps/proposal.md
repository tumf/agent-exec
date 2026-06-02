---
change_type: implementation
priority: high
dependencies: []
references:
  - src/main.rs
  - src/list.rs
  - src/schema.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
---

# Fix stale running jobs in ps output

**Change Type**: implementation

## Problem / Context

`agent-exec ps` is implemented as `list --state running`. Today it can report a persisted `running` job as live when the supervisor failed to write terminal state and the recorded process no longer exists. Operators can then see duplicate long-running services with identical commands and tags even though only one process tree is actually alive.

The observed stale shape is a job with `state.json` status `running`, a dead `pid`, no matching `_supervise --job-id <id>` process, and no recent log/state activity.

## Proposed Solution

Reconcile `running` jobs during `list`/`ps` presentation by validating the persisted PID before applying state filtering. A job whose persisted status is `running` but whose recorded PID is absent or no longer alive must not be presented as `running`.

The minimal implementation should classify such jobs as `unknown` in `list` output and therefore exclude them from `ps` and `list --state running`. This preserves the existing JSON schema and state vocabulary without mutating `state.json` during listing.

## Acceptance Criteria

- `agent-exec ps` and `agent-exec ps --all` do not include jobs whose `state.json` says `running` but whose persisted `pid` is dead or missing.
- `agent-exec list` and `agent-exec list --all` still show such stale jobs, but report `jobs[].state` as `unknown` instead of `running`.
- Live running jobs continue to appear as `running` and remain visible through `ps`.
- `list --state running` applies filtering after stale reconciliation, so stale jobs are excluded.
- The command output remains JSON-only and retains the existing response shape.

## Explicit Completion Conditions

- `src/list.rs` or an equivalent list-state path performs OS process liveness validation for persisted `running` states before constructing `JobSummary.state` and before applying `--state` filters.
- Unix/macOS liveness checking treats `kill(pid, 0)` success and `EPERM` as alive, and treats missing process errors as dead.
- Windows liveness checking uses process query APIs to distinguish `STILL_ACTIVE` from dead or inaccessible processes.
- Integration tests in `tests/integration.rs` cover a fake stale `running` job with a dead PID and prove `ps --all` excludes it while `list --all` reports it as `unknown`.
- Existing list/ps integration tests continue to pass for live running and exited jobs.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Persisting a new terminal status to `state.json` during `list`/`ps`.
- Adding new public state values such as `stale` or `orphaned`.
- Full supervisor-process discovery by scanning process tables for `_supervise --job-id <id>`.
- Validating that the PID still belongs to the original command tree beyond basic process liveness.
