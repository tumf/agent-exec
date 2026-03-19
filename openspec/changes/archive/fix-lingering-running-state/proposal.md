# Change Proposal: fix-lingering-running-state

## Problem / Context

`agent-exec` can keep a job in `running` state after the wrapped command has already completed successfully. In the reported issue, `tail` already shows successful completion messages, while `status` still reports `running` and lingering `_supervise` / wrapped command processes remain visible.

The current supervisor flow waits for log reader threads and other post-exit work before it persists the terminal state. If stdout/stderr remain open because a descendant process inherited the pipes, the supervisor can stay blocked even though the wrapped root process already exited.

## Proposed Solution

Treat wrapped-command exit as the source of truth for job completion. After the wrapped root process exits, the supervisor should persist a terminal state immediately, so `status` and `wait` stop reporting `running` without depending on pipe closure from descendants.

Keep output capture and completion notification best-effort, but do not let those follow-up activities block terminal-state persistence. Add regression coverage for a process tree that leaves inherited stdout/stderr open after the parent exits.

## Acceptance Criteria

- A job transitions out of `running` as soon as the wrapped root process exits, even if descendant processes still hold inherited stdout/stderr handles.
- `agent-exec status <job-id>` no longer reports `running` indefinitely for the reproduced issue shape.
- `agent-exec wait <job-id>` observes the terminal state without depending on log-reader thread shutdown.
- `_supervise` exits promptly after marking the job terminal instead of lingering behind a completed command.
- Integration tests cover the regression scenario and pass using the existing test harness.

## Out of Scope

- Repairing stale `running` jobs created by older versions.
- Adding a new public job state such as `orphaned` or `unknown` for this bug.
- Redesigning notification delivery beyond ensuring it does not block terminal-state persistence.
