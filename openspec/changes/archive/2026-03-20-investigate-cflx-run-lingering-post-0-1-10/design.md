# Design: investigate-cflx-run-lingering-post-0-1-10

## Summary

The post-`0.1.10` issue differs from the already-addressed lingering-state bug. The new reproduction uses the real workload `cflx run`, not a synthetic shell background process, and it demonstrates that visible success output is not currently sufficient evidence that the wrapped workload has actually exited.

## Reproduced Evidence

Local reproduction from this repository:

- launch: `agent-exec run --snapshot-after 0 -- cflx run`
- returned `job_id`: `01KM53EPAWS050A20W924ACXHJ`
- `tail` showed:
  - `No changes found for parallel execution`
  - `Orchestrator completed successfully`
- `status` still returned `running` immediately and again after 30 seconds
- process table showed:
  - `agent-exec _supervise` still alive
  - child `cflx run` still alive

This is strong evidence that the remaining bug is not merely "descendants keep pipe handles open after the wrapped root process exits." In this reproduction, the monitored workload process itself still exists.

## Why Existing Proposals Are Insufficient Alone

- `fix-lingering-running-state` addressed delayed terminal-state persistence after `child.wait()`.
- `fix-shell-wrapper-exec-handoff` addresses one plausible workload-boundary problem for argv launches.

Neither proposal, by itself, proves that `cflx run` is actually exiting in the failing reproduction. The reproduced evidence therefore requires a separate investigation track.

## Investigation Questions

1. Does `cflx run` intentionally remain alive after emitting its apparent success lines?
2. If yes, is the remaining lifetime meaningful work or merely shutdown lag?
3. If no, is `agent-exec` observing the wrong process boundary or failing to notice the true terminal condition?
4. Can the issue be modeled in an integration test without depending on an installed external `cflx` binary, or does it require a repository-local stand-in that reproduces the same lifecycle shape?

## Code Path Audit: What Evidence Is Required Before `running` Can Exit

### State Model

`running` is a value stored in `state.json` (`JobStatus::Running`). It is written once at job creation (`src/run.rs:215`) and transitions to a terminal value (`exited`, `killed`, or `failed`) at exactly one code path: `src/run.rs:1307-1351`.

```
child.wait()   ← blocks until root workload process exits (run.rs:1307)
 │
 └─> write terminal state to state.json (run.rs:1351)
      └─> status/wait see terminal state on next poll
```

### src/status.rs

`execute()` reads `state.json` directly and returns its `status` field verbatim. There is no process liveness probe, no log inspection, and no heuristic based on output content. If `state.json` says `running`, `status` returns `running`.

### src/wait.rs

`execute()` polls `state.json` in a loop using `state.status().is_non_terminal()` (line 50) as the exit condition. `is_non_terminal()` returns `true` for `created` and `running` only. Like `status`, it has no process probe and no log pattern matching.

### The Sole Transition Trigger

`running` can only transition to a terminal state when `child.wait()` returns in the supervisor (`src/run.rs:1307`). `child.wait()` blocks on the OS until the root workload process (the direct child of `_supervise`) actually exits. There is no other code path that writes a terminal status.

### Process Liveness vs. Log Completion

Log output such as "Orchestrator completed successfully" is captured by log-reader threads, written to `stdout.log`, and visible through `tail`. It does not trigger any state transition. The log and the process lifecycle are independent. The sequence in the reproduced case:

1. `cflx run` emits success-like lines → captured by log thread → visible in `tail`
2. `cflx run` remains alive → `child.wait()` has not returned → `state.json` still shows `running`
3. `status` returns `running` — correctly, because the process has not exited

Therefore "success in logs" is not sufficient evidence that `running` can be trusted to transition. The only trusted evidence is `child.wait()` returning, which requires the root workload process to have exited.

## Ownership Conclusion

The evidence from the code path audit and the reproduction leads to a single conclusion:

**The fix belongs in `cflx`.**

Agent-exec's state model is correct: `running` accurately reflects that the monitored process is still alive. The mismatch is that `cflx run` emits completion output but does not exit promptly after its orchestration work finishes. A workload that prints "Orchestrator completed successfully" and then lingers for an indeterminate period creates an observable gap between "apparent success in logs" and "job marked done." That gap is the bug.

Possible `cflx`-side root causes to investigate (in priority order):

1. `cflx run` performs non-trivial post-orchestration cleanup (cleanup that delays exit) — if so, it should either move cleanup earlier, make it non-blocking, or suppress it in the no-work case.
2. `cflx run` holds async runtimes or background tasks open after its main work completes — if so, it must join or cancel them before returning from main.
3. `cflx run` itself spawns child processes that hold it alive (e.g., sub-agents, shell hooks) — if so, `cflx` must wait for and reap those children before exiting.

If investigation of `cflx` reveals that the lingering time is genuinely shorter than the `agent-exec` polling interval (200 ms by default) and the issue is a race, then a coordinated fix involving `agent-exec` retry logic may be warranted. That scenario should be ruled out with timing instrumentation before concluding that both sides need changes.

## Expected Outcome

This proposal ends with the ownership conclusion above: **`cflx` must exit promptly after emitting its success lines.** Future fixes must demonstrate that `agent-exec run -- cflx run` transitions to a terminal state within the same bounded window as `agent-exec run -- sh -c 'echo done'` (i.e., within 200–500 ms of the workload printing its final line), not only within the longer 2-second log-drain window.
