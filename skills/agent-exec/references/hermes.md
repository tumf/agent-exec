# Hermes Agent Integration

Use this reference when `agent-exec` jobs must report completion back into a Hermes Agent session via `hermes notify`.

## How it works

1. `agent-exec run` starts a background job.
2. On completion, `--notify-command` invokes `hermes notify`.
3. `hermes notify` spins up a one-shot Hermes Agent that reads the notification payload, interprets it (may inspect files, logs, etc.), and delivers a human-readable response to the origin chat.

This is different from posting a raw message to Telegram — the agent processes the result before responding.

## Prerequisites

- `hermes notify` is available at `~/.hermes/hermes-agent/venv/bin/hermes notify` (or on PATH).
- A valid LLM provider must be reachable. Pass `--provider` explicitly to avoid credential resolution hangs in non-interactive environments.
- The Telegram bot token (or other platform credentials) must be configured in `~/.hermes/.env`.

## Pass session context via environment variables

`hermes notify` resolves its delivery target from `HERMES_SESSION_*` environment variables. Inject them with `--env` at job creation time:

```bash
agent-exec run \
  --env HERMES_SESSION_PLATFORM=telegram \
  --env HERMES_SESSION_CHAT_ID=<chat_id> \
  --env HERMES_SESSION_THREAD_ID=<thread_id> \
  --notify-command 'hermes notify --provider <provider> -m "job $AGENT_EXEC_JOB_ID completed"' \
  -- <command>
```

The `--env` variables are inherited by the notify-command process, so `hermes notify` picks them up automatically without needing `--platform`/`--chat-id`/`--thread-id` flags.

## Recommended notify-command shape

Keep the one-liner minimal. The agent will inspect logs and files on its own.

```bash
--notify-command 'hermes notify --provider <provider> -m "job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"'
```

For richer context, include the command description:

```bash
--notify-command 'hermes notify --provider <provider> -m "Build finished: job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"'
```

## Example: full launcher

```bash
HERMES=~/.hermes/hermes-agent/venv/bin/hermes

agent-exec run \
  --env HERMES_SESSION_PLATFORM=telegram \
  --env HERMES_SESSION_CHAT_ID=971980613 \
  --env HERMES_SESSION_THREAD_ID=27136 \
  --notify-command "$HERMES notify --provider cliproxy -m 'Build done: job_id=\$AGENT_EXEC_JOB_ID event_path=\$AGENT_EXEC_EVENT_PATH'" \
  -- make build
```

## Example: attach notification after launch

```bash
JOB=$(agent-exec run --snapshot-after 0 -- make test | jq -r .job_id)

agent-exec notify set "$JOB" \
  --command 'hermes notify --provider cliproxy -m "Tests finished: job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"'
```

Use this when session context is only available after the job starts.

## Helper script

For repeated use, create a wrapper script instead of inlining the full command:

```bash
#!/usr/bin/env bash
# ~/.local/bin/hermes-notify-hook
# Usage: agent-exec run --notify-command hermes-notify-hook -- <command>
set -euo pipefail

HERMES="${HERMES_BIN:-$HOME/.hermes/hermes-agent/venv/bin/hermes}"
PROVIDER="${HERMES_NOTIFY_PROVIDER:-cliproxy}"

exec "$HERMES" notify \
  --provider "$PROVIDER" \
  -m "job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"
```

Then launch jobs simply:

```bash
agent-exec run \
  --env HERMES_SESSION_PLATFORM=telegram \
  --env HERMES_SESSION_CHAT_ID=<chat_id> \
  --env HERMES_SESSION_THREAD_ID=<thread_id> \
  --notify-command hermes-notify-hook \
  -- <command>
```

## Good patterns

- Always pass `--provider` explicitly to avoid interactive credential prompts in headless contexts.
- Use `--env` to inject `HERMES_SESSION_*` variables so notify-command inherits them automatically.
- Include `job_id` and `event_path` in the message so the agent can inspect the completion event and job logs.
- Keep the one-liner idempotent — duplicate delivery attempts should not cause side effects.
- Use absolute paths for the `hermes` binary when PATH may differ inside the sink process.

## Common mistakes

- Omitting `--provider`, causing `hermes notify` to hang on interactive credential resolution.
- Hardcoding `--thread-id` in `--notify-command` instead of using `HERMES_SESSION_THREAD_ID` via `--env`.
- Sending the full event JSON as the message payload. The agent can read files — just pass `event_path`.
- Using a model/provider that is slow or unreliable for one-shot callbacks. Prefer fast, cheap models.
- Forgetting that `hermes notify` starts a full agent turn — it is not a simple HTTP POST. Budget ~10-30s for completion.
