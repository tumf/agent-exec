---
name: agent-exec
description: Use agent-exec for shell work whose duration or output is uncertain, especially when an MCP client can start and observe a detached managed job. Apply the lifecycle rules here instead of treating transport timeouts, bounded waits, or missing inline output as cancellation.
---

# agent-exec

Use agent-exec for non-trivial shell work so execution remains detached, observable, and recoverable.

## Principles

- Prefer the MCP tools when available. Start with `run` and retain the returned `job_id`.
- Treat `status`, `tail`, and bounded `wait` as observation only. Transport closure, wait expiry, missing output, or moving to other work does not stop the job.
- Use further observation only for an explicit progress request, result collection, or diagnosis.
- Call `kill` only after an explicit cancellation request.
- Treat `notification.state="armed"` with `polling_required=false` as proof that the sink was persisted, not that downstream delivery succeeded. Do not add duplicate polling or a watcher.
- A response without notification state proves nothing about completion routing. Keep an observation path until terminal state is verified.
- Verify completion from terminal state, exit status, logs, and the requested artifact. A callback or watcher firing is not workload success.
- Use an inline shell call only when the command is clearly short, blocking, and safe to finish in the current response.

## References

- Read `references/cli-contract.md` when MCP is unavailable or CLI syntax, defaults, JSON, exit codes, configuration, or observation budgets matter.
- Read `references/completion-events.md` for completion payloads and sink delivery records.
- Read `references/hermes.md` or `references/openclaw.md` only for that host's callback and fallback-watcher rules.
