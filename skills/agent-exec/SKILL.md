---
name: agent-exec
description: Use `agent-exec` whenever shell work may run longer than a safe inline Bash call, may produce more output than you want in context, or should remain observable as a managed job. Start with plain `agent-exec run -- <command>` and rely on its default behavior; do not add custom wait or timeout flags unless there is a concrete reason. This skill is especially relevant when the user wants to run builds, tests, servers, data jobs, or any command whose duration or output size is uncertain.
---

# agent-exec

Use `agent-exec` as the default harness-friendly way to run shell work whose duration or output size is not trivial.

For MCP-capable clients, configure `agent-exec mcp` as a stdio server and call its `run` tool first. It starts the same detached managed jobs without routing lifecycle through the client terminal. Retain each returned `job_id`; observe with `status`, `tail`, or bounded `wait`. Call `kill` only after an explicit user cancellation request.

When MCP is unavailable, start with plain `agent-exec run -- <command>`. In normal use, do not try to outsmart it with extra flags. The defaults are the point: they are chosen so the harness gets control back predictably, sees common startup failures early, and avoids flooding context with command output.

Use a normal inline shell command only when the task is clearly short, blocking, and safe to finish within one response.

## Hermes Native MCP configuration

```yaml
mcp_servers:
  agent-exec:
    command: agent-exec
    args: ["mcp"]
```

Set `args: ["--root", "/path/to/jobs", "mcp"]` only when a non-default jobs root is required. MCP transport closure, bounded wait expiry, missing output, and moving to other work are not cancellation authorization.

## MCP observation budget

Each MCP host must provide its own already-safe observation value through `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`. The value becomes the default and maximum `until` for MCP `run` and `wait`; agent-exec does not calculate a client timeout or safety margin.

OpenCode's current 60-second MCP request deadline can use `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`. Hermes and other hosts must independently choose and pass their own safe value.

## Why use it by default

- It returns within a bounded default wait window instead of letting an uncertain command consume the whole harness timeout.
- It waits just enough to surface common early failures, which often removes the extra round trip of `run` and then immediately `status` or `tail`.
- It returns only a partial inline view of stdout and stderr, so large output does not dominate context.
- It persists full logs and returns machine-readable JSON, so follow-up inspection is reliable.

## Default posture

- For typical use, run `agent-exec run -- <command>` and stop there.
- Pass the workload as normal argv after `--`. Do not wrap it in `sh -lc` yourself unless you specifically need shell syntax such as pipes, redirects, variable expansion, or compound commands.
- Do not add wait-related flags unless you have a concrete reason.
- Do not optimize around output volume yourself; inspect the returned log paths when you need the full output.
- Treat the JSON response as the interface. Avoid wrapping it with extra stdout text.

## Exceptions

- Use `--no-wait` only for fire-and-forget cases where immediate return matters more than seeing startup output.
- Use `wait --forever` or similar explicit blocking only when you deliberately want to stay attached until completion.
- Use custom timing, notification, masking, or shell-wrapper options only when the task actually needs them.

## Read more only when needed

- Read `references/cli-contract.md` for the response schema, exit codes, and default `run` behavior.
- Read `references/completion-events.md` for `stdout_log_path`, `stderr_log_path`, and notification sink behavior.
- Read `references/openclaw.md` when job completion should re-enter an OpenClaw workflow.
- Read `references/hermes.md` when job completion should notify a Hermes Agent session.

## Minimal examples

```bash
agent-exec run -- make test
agent-exec run -- npm run build
agent-exec run -- cargo test
```

If the command keeps running, use the returned `job_id` plus the references above to inspect, wait, tail, notify, or kill as needed.
