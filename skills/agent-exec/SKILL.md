---
name: agent-exec
description: Run and manage non-interactive background jobs with the `agent-exec` CLI. Use when Claude needs to run a command in the background, return a job id immediately, poll or wait for completion, tail logs, inspect or kill jobs, list jobs by state or working directory, install the built-in skill, or trigger job-finished notifications via `job.finished` events.
---

# agent-exec

Use `agent-exec` when a command should run as a managed job instead of an inline shell process.

## Follow these rules

- Keep stdout machine-readable. `agent-exec` prints one JSON object to stdout; diagnostic logs belong on stderr.
- Prefer `agent-exec run` for long-running or pollable work, then use `status`, `tail`, `wait`, `kill`, or `list` as needed.
- Use `--wait` when the caller needs terminal state in the initial response.
- Use `--mask KEY` for secrets passed via `--env`; masked values appear as `***` in JSON and persisted metadata.
- Use `--notify-command` or `--notify-file` when another process must react to job completion.

Read `references/cli-contract.md` when you need the exact response envelope, exit-code behavior, or `run`/`list` contract details.

Read `references/completion-events.md` when you need the `job.finished` payload shape, sink behavior, or `completion_event.json` semantics.

## Run a job

```bash
agent-exec run [OPTIONS] -- <COMMAND> [ARGS...]
```

Use these options most often:

- `--snapshot-after <ms>`: delay the initial response briefly to include a snapshot
- `--timeout <ms>` / `--kill-after <ms>`: enforce termination deadlines
- `--cwd <dir>`: run from a specific directory
- `--env KEY=VALUE` / `--env-file <file>`: set environment variables
- `--no-inherit-env`: avoid inheriting the current process environment
- `--wait` / `--wait-poll-ms <ms>`: return only after the job reaches a terminal state
- `--notify-command <COMMAND>`: run a shell command on completion; executed via `sh -lc` (Unix) or `cmd /C` (Windows); event JSON is sent to stdin
- `--notify-file <PATH>`: append one NDJSON `job.finished` event per completed job

Pass a plain shell command string to `--notify-command`. The command sink also receives:

- `AGENT_EXEC_EVENT_PATH`
- `AGENT_EXEC_JOB_ID`
- `AGENT_EXEC_EVENT_TYPE`

Choose the sink based on who needs the event next:

- Use `--notify-command` when a small, fast, direct action should happen immediately after completion, such as posting to a chat, calling a webhook helper, or routing the event back to a launcher session.
- Use `--notify-file` when another durable worker should consume events later, retries matter, or several downstream systems may need the same event.
- Prefer checked-in helper scripts over large inline shell or Python snippets so quoting, dependencies, and reply-target logic stay reviewable.

## Inspect a job

Use these commands after `run`:

- `agent-exec status <JOB_ID>`: read current state (`running`, `exited`, `killed`, `failed`)
- `agent-exec tail [--tail-lines N] [--max-bytes N] <JOB_ID>`: read stdout/stderr tails
- `agent-exec wait [--timeout-ms N] [--poll-ms N] <JOB_ID>`: block until terminal state
- `agent-exec kill [--signal TERM|INT|KILL] <JOB_ID>`: request termination

## List jobs

```bash
agent-exec list [--state running|exited|killed|failed|unknown] [--cwd DIR] [--all] [--limit N]
```

- By default, `list` filters to the caller's current working directory.
- Use `--cwd <dir>` to filter by a specific directory.
- Use `--all` to disable cwd filtering.

## Handle completion events

Read `references/completion-events.md` for the full `job.finished` payload, sink environment variables, and persistence details.

Suggested patterns:

- Notify a chat or session directly: use `--notify-command` with a checked-in helper that reads event JSON from stdin and delivers it with the OpenClaw entrypoint that fits the case, such as `openclaw message send` or `openclaw agent --session-id ... --deliver`.
- Return the event to the launching OpenClaw session: use `--notify-command` to call a helper that forwards the event to the original session or conversation id; both `message` and `agent --deliver` can be valid depending on whether you want lightweight delivery or explicit agent re-entry.
- Append to a file for a durable worker: use `--notify-file` when a separate process should handle retries, fanout, or slower downstream APIs.

Operational reminders:

- Notification delivery is best effort. Sink failure does not change the main job state.
- Check `completion_event.json.delivery_results` when delivery success matters.
- Keep notify commands idempotent and quick; long or fragile sink logic belongs in a script or worker.

## Install the built-in skill

```bash
agent-exec install-skills [--source self|local:<path>] [--global]
```

- Use `self` to install the built-in `agent-exec` skill.
- Use `local:<path>` to install a local skill directory.
- Expect installation into `./.agents/skills/` by default or `~/.agents/skills/` with `--global`.
- Expect `.agents/.skill-lock.json` to be created or updated.

## Respect the contract

- Treat exit code `0` as success.
- Treat exit code `1` as an expected failure with a JSON error envelope on stdout.
- Treat exit code `2` as a clap usage error.
- Do not emit extra stdout text around `agent-exec` responses.
- Read `references/cli-contract.md` before changing assumptions about response fields.
