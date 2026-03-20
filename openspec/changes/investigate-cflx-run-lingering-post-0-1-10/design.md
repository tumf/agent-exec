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

## Expected Outcome

This proposal should end with one of three concrete conclusions:

- `agent-exec` must change how it defines or reconciles `running`
- `cflx` must change its shutdown behavior
- both sides need coordinated fixes

The important design constraint is that future fixes must be evaluated against the reproduced `cflx run` behavior, not only against synthetic shell cases.
