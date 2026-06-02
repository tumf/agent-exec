# Design: list-time stale running reconciliation

## Current Behavior

`ps` delegates to the list command with `state=running`. The list command reads `meta.json` and `state.json`, constructs `JobSummary.state` from persisted job status, then applies the state filter. If a supervisor exits before writing terminal state, a stale `running` status can remain forever.

## Design Decision

Perform non-mutating reconciliation at list presentation time:

1. Read persisted `JobState`.
2. If the persisted status is not `running`, keep the persisted status.
3. If the persisted status is `running`, validate `state.pid`.
4. If the PID is missing or not alive, expose the effective state as `unknown`.
5. Apply `--state` filtering to the effective state.

This keeps persisted state untouched and avoids changing public JSON schema fields.

## Platform Liveness Semantics

- Unix/macOS: `kill(pid, 0)` success means alive; `EPERM` means alive but not signalable; other failures mean not alive.
- Windows: opening the process for limited query and checking `GetExitCodeProcess == STILL_ACTIVE` means alive.
- Unsupported platforms: preserve persisted state to avoid false stale classification.

## Alternatives Considered

### Add `stale` or `orphaned` public state

This is more expressive but changes the state vocabulary, CLI parser validation, schema comments, and downstream expectations. It is out of scope for the minimal bugfix.

### Mutate `state.json` during `ps`

This makes listing have side effects and risks rewriting jobs based on transient process-query failures. The selected design keeps `list`/`ps` read-only.

### Scan for `_supervise --job-id <id>`

This could detect orphaned supervisors or child-only cases, but it is platform-specific and currently impossible to do precisely from persisted metadata because `state.pid` is overwritten from supervisor PID to child PID. It can be proposed later if needed.
