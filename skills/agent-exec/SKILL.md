---
name: agent-exec
description: Use agent-exec for shell work whose duration or output is uncertain, especially when an MCP client can start and observe a detached managed job. Apply the lifecycle rules here instead of treating transport timeouts, bounded waits, or missing inline output as cancellation.
---

# agent-exec

Use agent-exec for non-trivial shell work so execution remains detached, observable, and recoverable.

## Principles

- Prefer the MCP tools when available. Start with `run` and retain the returned `job_id`.
- Treat `status`, `tail`, and bounded `wait` as observation only. Transport closure, wait expiry, missing output, or moving to other work does not stop the job.
- Never set the `abandon_job_after` launch field or `--abandon-job-after` flag as a safety default. It gives up on the job, terminates it, and can permanently lose unfinished results; it is not an observation deadline. It is destructive enough to require `acknowledge_result_loss=true` (`--acknowledge-result-loss`), so set it only on an explicit instruction to abandon the job after a deadline.
- The old `timeout` field and `--timeout` flag are removed. They now always fail with migration guidance and never launch a job; do not try to reintroduce them.
- When a job was actually abandoned, its result carries `abandoned_by="abandon_job_after"` and `result_loss=true` in `state.json`, `status`, `list`, and the completion event. Treat those markers as evidence that unfinished results may be missing, not as normal completion.
- Use `until` when the caller only needs control returned after a bounded observation period. `until` must leave the job running.
- Call `kill` only after an explicit cancellation request. Do not infer cancellation from elapsed time, client timeout, inactivity, missing output, or an estimated completion time.
- Use further observation only for an explicit progress request, result collection, or diagnosis.
- Treat `notification.state="armed"` with `polling_required=false` as proof that the sink was persisted, not that downstream delivery succeeded. Do not add duplicate polling or a watcher.
- A response without notification state proves nothing about completion routing. Keep an observation path until terminal state is verified.
- Verify completion from terminal state, exit status, logs, and the requested artifact. A callback or watcher firing is not workload success.
- Use an inline shell call only when the command is clearly short, blocking, and safe to finish in the current response.

## References

- Read `references/cli-contract.md` when MCP is unavailable or CLI syntax, defaults, JSON, exit codes, configuration, or observation budgets matter.
- Read `references/completion-events.md` for completion payloads and sink delivery records.
- Read `references/hermes.md` or `references/openclaw.md` only for that host's callback and fallback-watcher rules.
