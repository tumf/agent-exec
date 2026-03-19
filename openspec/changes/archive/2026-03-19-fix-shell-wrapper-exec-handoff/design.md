# Design: fix-shell-wrapper-exec-handoff

## Summary

The unresolved part of issue `#5` comes from tying argv-style job execution to a shell process that remains distinct from the intended workload. The existing fix shortens post-exit cleanup after `child.wait()`, but if the monitored child is still `sh -lc ...`, completion semantics remain wrapper-centric.

## Current Behavior

- `run` always resolves a shell wrapper and launches the job through it.
- Multi-argument commands are converted into one quoted shell command string.
- The supervisor waits on the wrapper child PID.

That means `agent-exec run -- cflx run` does not currently make the monitored child process become `cflx run`; it monitors the shell wrapper that launched it.

## Proposed Unix Behavior

Split launch semantics by command shape:

- `command.len() == 1`: keep the current shell-string path.
- `command.len() > 1`: invoke the wrapper with an `exec "$@"` script body and pass the argv payload separately.

Conceptually, argv mode becomes equivalent to:

```sh
sh -lc 'exec "$@"' sh cflx run
```

The shell still performs the launch, but it replaces itself with the argv workload. The monitored child PID therefore tracks the intended workload boundary on Unix.

## Why This Is Preferred

- It keeps the configured shell wrapper in use, matching the user's stated constraint.
- It avoids brittle process-table inference after launch.
- It preserves shell-string behavior where shell syntax is the intended workload.
- It keeps completion semantics aligned with the actual argv workload for common cases like `agent-exec run -- cflx run`.

## Verification Strategy

- Add Unix integration coverage that distinguishes argv mode from string mode.
- Confirm argv mode still works with the default wrapper and configurable wrappers.
- Confirm string mode still supports shell features such as `&&` unchanged.
- Add or adapt a regression test tied to issue `#5` so the workload boundary is no longer a lingering shell parent in argv mode.
