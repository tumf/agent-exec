# Change Proposal: fix-shell-wrapper-exec-handoff

## Problem / Context

Issue `#5` remains only partially addressed. The recent fix handles the case where descendants keep inherited stdout/stderr open after the wrapped root process exits, but it does not address runs where the shell wrapper itself is still the long-lived process boundary.

Current Unix execution always launches job commands through the configured shell wrapper and passes the workload as a shell command string. For argv-style invocations such as `agent-exec run -- cflx run`, this means the effective child is still `sh -lc '<quoted argv>'` rather than a direct `cflx run` process. That keeps completion semantics tied to the wrapper instead of the intended workload.

## Proposed Solution

Preserve shell-wrapper usage for argv-style invocations, but change the Unix launch path so multi-argument commands use an `exec "$@"` handoff pattern inside the wrapper. In practice, the shell still starts first, but it replaces itself with the target argv workload instead of remaining as a distinct long-lived parent process.

Keep single-string command mode unchanged: string commands remain shell-native and continue to treat the wrapper process as the workload boundary. Only argv-style invocations gain the `exec` handoff.

## Acceptance Criteria

- On Unix-like platforms, argv-style job launches (for example `agent-exec run -- cflx run`) still use the resolved shell wrapper but hand off to the target workload via `exec`, so the observed child lifecycle matches the target argv command rather than a lingering shell parent.
- Single-string command mode (for example `agent-exec run -- 'echo hi && echo bye'`) preserves current shell-string semantics.
- The resolved shell wrapper remains shared between job execution and `--notify-command`; this proposal changes only argv-style workload launch semantics.
- Integration coverage distinguishes argv-style execution from string-command execution and verifies the Unix `exec` handoff behavior.
- Documentation/spec text explains that shell-wrapper usage remains in place, but argv mode uses an `exec` handoff so completion tracking aligns with the wrapped workload.

## Out of Scope

- Redesigning Windows command launch semantics.
- Adding process-table heuristics (`ps`/`pgrep`) to infer the main workload PID after launch.
- Changing notify-command delivery semantics beyond clarifying that it still uses the resolved shell wrapper.
