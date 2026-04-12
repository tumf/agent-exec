---
name: agent-exec
description: Run and manage non-interactive background jobs with the `agent-exec` CLI. Use when Claude needs to run a command in the background, return a job id immediately, poll or wait for completion, tail logs, inspect or kill jobs, list jobs by state or working directory, install the built-in skill, or trigger job-finished notifications via `job.finished` events.
---

# agent-exec

Use `agent-exec` when a command should run as a managed job instead of an inline shell process.

Choose it when work should outlive the current turn, return a job id immediately, be polled later, or trigger a completion hook. Keep using a normal shell command for short, blocking tasks that should finish in one response.

## Follow these rules

- Keep stdout machine-readable. `agent-exec` prints one JSON object to stdout; diagnostic logs belong on stderr.
- Prefer `agent-exec run` for long-running or pollable work, then use `status`, `tail`, `wait`, `kill`, or `list` as needed.
- Use `--wait` when the caller needs terminal state in the initial response.
- Use `--mask KEY` for secrets passed via `--env`; masked values appear as `***` in JSON and persisted metadata.
- Use `--notify-command` or `--notify-file` when another process must react to job completion.

Read `references/cli-contract.md` when you need the exact response envelope, exit-code behavior, or `run`/`list` contract details.

Read `references/completion-events.md` when you need the `job.finished` payload shape, sink behavior, or `completion_event.json` semantics.

Read `references/openclaw.md` when a job completion should be routed back into an OpenClaw chat, user flow, or agent session.

Read `references/hermes.md` when a job completion should notify a Hermes Agent session (e.g. deliver interpreted results to Telegram via `hermes notify`).

## Typical workflow

1. Start the job with `agent-exec run`.
2. Capture the returned `job_id`.
3. Use `status` or `wait` for lifecycle state.
4. Use `tail` to inspect output without breaking the JSON-only contract.
5. Use `kill` when the job should stop early.

## Run a job

```bash
agent-exec run [OPTIONS] -- <COMMAND> [ARGS...]
```

Use these options most often:

- `--tail-lines <N>` / `--max-bytes <N>`: size the returned snapshot tails (defaults: `50`, `65536`)
- `--timeout <ms>` / `--kill-after <ms>`: enforce termination deadlines (defaults: `0`, `0`)
- `--cwd <dir>`: run from a specific directory (default: the caller's current working directory)
- `--env KEY=VALUE` / `--env-file <file>`: set environment variables
- `--no-inherit-env`: avoid inheriting the current process environment (default behavior is to inherit it)
- `--wait`: return only after the job reaches a terminal state
- `--until <seconds>` / `--forever`: bound or remove the client-side wait deadline when `--wait` is used (default: `30`; without `--wait`, `run` does not use this deadline)
- `--notify-command <COMMAND>`: run a shell command on completion via the configured shell wrapper (default wrapper: `sh -lc` on Unix, `cmd /C` on Windows); event JSON is sent to stdin
- `--notify-file <PATH>`: append one NDJSON `job.finished` event per completed job
- `--config <PATH>`: load shell wrapper settings from a specific `config.toml`
- `--shell-wrapper <PROG FLAGS>`: override the shell wrapper for this invocation (e.g. `"bash -lc"`); applies to both command-string execution and `--notify-command`

Default behavior for `run`:

- without `--wait`, returns after a short snapshot wait instead of waiting for completion: `--snapshot-after 10000`
- includes up to `50` tail lines and `65536` bytes per stream in snapshots
- does not enforce a runtime limit unless `--timeout` is set
- runs in the caller's current working directory unless `--cwd` is set
- inherits the caller's environment unless `--no-inherit-env` is set
- does not wait for terminal state unless `--wait` is set
- with `--wait`, does not use `--snapshot-after`; instead it uses a 30 second client-side wait deadline unless `--until` or `--forever` changes that

Pass a plain shell command string to `--notify-command`. The command sink also receives:

- `AGENT_EXEC_EVENT_PATH`
- `AGENT_EXEC_JOB_ID`
- `AGENT_EXEC_EVENT_TYPE`

Choose the sink based on who needs the event next:

- Use `--notify-command` when a small, fast, direct action should happen immediately after completion, such as posting to a chat, calling a webhook helper, or routing the event back to a launcher session.
- Use `--notify-file` when another durable worker should consume events later, retries matter, or several downstream systems may need the same event.
- Prefer a compact one-liner for agent-authored OpenClaw callbacks; move to a separate script only when quoting or branching becomes hard to keep correct.

## Inspect a job

Use these commands after `run`:

- `agent-exec status <JOB_ID>`: read current state (`running`, `exited`, `killed`, `failed`)
- `agent-exec tail [--tail-lines N] [--max-bytes N] <JOB_ID>`: read stdout/stderr tails (defaults: `50`, `65536`)
- `agent-exec wait [--until N] [--poll N] [--forever] <JOB_ID>`: block until terminal state (defaults: `--poll 1`, `--until 30` unless `--forever` is set)
- `agent-exec kill [--signal TERM|INT|KILL] <JOB_ID>`: request termination (default signal: `TERM`)
- `agent-exec notify set <JOB_ID> --command <COMMAND>`: attach or replace the completion callback after the job has already started

Use `wait` when the caller needs a terminal outcome before proceeding. Use `status` when the job should continue running while the caller does other work.

## List jobs

```bash
agent-exec list [--state running|exited|killed|failed|unknown] [--cwd DIR] [--all] [--limit N]
```

- By default, `list` filters to the caller's current working directory.
- Use `--cwd <dir>` to filter by a specific directory.
- Use `--all` to disable cwd filtering.

## Handle completion events

Read `references/completion-events.md` for the full `job.finished` payload, sink environment variables, and persistence details.

Read `references/openclaw.md` when the completion path should re-enter OpenClaw.

Suggested patterns:

- Return the event to the launching OpenClaw session: use `--notify-command` to forward the event to the original session or conversation id with `openclaw agent --deliver --reply-channel ... --session-id ... -m ...`.
- Deliver interpreted results via Hermes Agent: use `--notify-command` with `hermes notify` to spin up a one-shot agent that reads the payload, inspects logs/files, and posts a human-readable summary to the origin chat. Read `references/hermes.md` for setup and examples.
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
- Expect the success payload to include installed skill summaries plus `lock_file_path`.

## Respect the contract

- Treat exit code `0` as success.
- Treat exit code `1` as an expected failure with a JSON error envelope on stdout.
- Treat exit code `2` as a clap usage error.
- Do not emit extra stdout text around `agent-exec` responses.
- Read `references/cli-contract.md` before changing assumptions about response fields.
