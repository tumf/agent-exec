# Change Proposal: investigate-cflx-run-lingering-post-0-1-10

## Problem / Context

Issue `#5` remains reproducible after the `0.1.10` line of fixes. A direct local reproduction now exists using the real workload:

- command: `agent-exec run --snapshot-after 0 -- cflx run`
- observed `job_id`: `01KM53EPAWS050A20W924ACXHJ`
- `tail` quickly reports `No changes found for parallel execution` and `Orchestrator completed successfully`
- `status` still reports `running` immediately afterward and again 30 seconds later
- `ps` shows both `agent-exec _supervise` and `cflx run` still present

This means the remaining issue is not limited to the already-fixed case where descendants merely keep inherited stdio open after the wrapped root process exits. In this reproduction, the wrapped workload process itself (`cflx run`) remains alive after logging apparent success.

## Proposed Solution

Create a focused post-`0.1.10` proposal that treats the reproduced `cflx run` behavior as the source-of-truth acceptance case. The implementation work should first capture this reproduction in integration coverage or equivalent repository-verifiable fixtures, then determine whether the remaining fix belongs in:

- `agent-exec` process-boundary tracking,
- `agent-exec` stale-running reconciliation, or
- the upstream `cflx run` lifecycle itself.

The immediate goal of this proposal is not to pick one speculative mechanism prematurely, but to make the reproduced failure impossible to regress or hand-wave away during future fixes.

## Acceptance Criteria

- The reproduced `agent-exec run --snapshot-after 0 -- cflx run` behavior is documented as the primary post-`0.1.10` acceptance case for issue `#5`.
- Repository verification includes a regression path that models the reproduced condition closely enough to fail while `status` remains stuck in `running` after visible orchestration success output.
- The resulting fix path explicitly distinguishes between at least these two cases: (1) root process already exited but descendants hold stdio open, and (2) the wrapped workload process itself remains alive after apparent success output.
- `status` / `wait` semantics for the final fix are defined against the reproduced workload behavior rather than only against synthetic shell-only reproductions.
- Any follow-up implementation or design notes state clearly whether the ultimate fix belongs in `agent-exec`, `cflx`, or both.

## Ownership

Code path audit of `src/status.rs`, `src/wait.rs`, and `src/run.rs` confirms that agent-exec's state model is correct: `running` is set at job creation and cleared only when `child.wait()` returns (i.e., when the root workload process exits). There is no heuristic based on log output, no process liveness probe, and no other transition path.

**The fix belongs in `cflx`.** `cflx run` must exit promptly after its orchestration work completes. The reproduction shows that it currently lingers after printing success lines. See `design.md` for the full code path audit, the specific evidence required before `running` can exit, and the ranked list of plausible `cflx`-side root causes.

If a follow-up investigation reveals a race between `cflx run`'s exit and agent-exec's polling interval, a coordinated fix may be needed. That scenario requires timing instrumentation to confirm before changing agent-exec behavior.

## Out of Scope

- Claiming that shell-wrapper `exec` handoff alone resolves issue `#5`.
- Closing the issue based only on synthetic `sh -lc` reproductions that do not exercise `cflx run`.
- Redesigning unrelated CLI surfaces or notification features.
