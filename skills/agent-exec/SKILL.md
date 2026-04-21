---
name: agent-exec
description: Use `agent-exec` whenever shell work may run longer than a safe inline Bash call, may produce more output than you want in context, or should remain observable as a managed job. Start with plain `agent-exec run -- <command>` and rely on its default behavior; do not add custom wait or timeout flags unless there is a concrete reason. This skill is especially relevant when the user wants to run builds, tests, servers, data jobs, or any command whose duration or output size is uncertain.
---

# agent-exec

Use `agent-exec` as the default harness-friendly way to run shell work whose duration or output size is not trivial.

Start with plain `agent-exec run -- <command>`. In normal use, do not try to outsmart it with extra flags. The defaults are the point: they are chosen so the harness gets control back predictably, sees common startup failures early, and avoids flooding context with command output.

Use a normal inline shell command only when the task is clearly short, blocking, and safe to finish within one response.

## Why use it by default

- It returns within a bounded default wait window instead of letting an uncertain command consume the whole harness timeout.
- It waits just enough to surface common early failures, which often removes the extra round trip of `run` and then immediately `status` or `tail`.
- It returns only a partial inline view of stdout and stderr, so large output does not dominate context.
- It persists full logs and returns machine-readable JSON, so follow-up inspection is reliable.

## Default posture

- For typical use, run `agent-exec run -- <command>` and stop there.
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
