# Design: fix-lingering-running-state

## Summary

The bug exists because supervisor completion is coupled to pipe-drain completion. The wrapped root command may exit, but `_supervise` currently waits for stdout/stderr reader threads to finish before it writes the terminal job state. If a descendant process inherits the same pipe endpoints, EOF may never arrive promptly, so the job remains externally visible as `running`.

## Desired Behavior

Terminal job state should be derived from wrapped root process exit, not from the lifecycle of descendant-held stdio handles. Once `child.wait()` returns, callers should be able to observe terminal state immediately through `status` and `wait`.

## Proposed Execution Order

1. Wait for the wrapped root process with `child.wait()`.
2. Derive terminal status, exit code, signal, duration, and `finished_at`.
3. Persist terminal `state.json` immediately.
4. Perform best-effort cleanup and completion-event delivery.
5. Exit `_supervise` promptly.

## Trade-offs

- This may allow a small amount of late descendant output to arrive after the job is already marked terminal.
- That trade-off is acceptable because the CLI contract relies on job state for completion detection, and a delayed terminal transition is worse than trailing log bytes arriving after completion.

## Verification Strategy

- Reproduce the issue shape with a short-lived parent command that spawns a descendant inheriting stdout/stderr.
- Assert that `status` becomes `exited` (or another terminal state, depending on the command result) shortly after the parent exits.
- Assert that `_supervise` itself no longer remains visible as a lingering process for that job.
