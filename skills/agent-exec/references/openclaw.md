# OpenClaw Integration

Use this reference when `agent-exec` jobs must report completion back into an OpenClaw workflow.

## Fit `agent-exec` into OpenClaw

- Start long-running work with `agent-exec run` when the current turn should return before the command finishes.
- Attach `--notify-command` when OpenClaw should react immediately after completion.

## Choose a delivery style

- Use `openclaw agent --deliver --reply-channel ... --session-id ... -m ...` when the completion event should explicitly re-enter an existing agent session.
- Prefer a compact inline `--notify-command` one-liner for agent-authored callbacks. Move to a separate script only when the command becomes too long or branches in several places.

## Recommended notify-command shape

Pass a small inline command string to `--notify-command` and let it:

1. Prefer `AGENT_EXEC_EVENT_PATH` when the downstream command can read a file directly.
2. Read stdin only when the downstream command truly needs the raw event stream.
3. Choose the correct OpenClaw entrypoint.
4. Emit only the minimum fields the receiving agent needs. On the same host, `job_id` plus `event_path` is a good default.

## Example one-liners

Example launcher command:

```bash
agent-exec run \
  --notify-command 'openclaw agent --deliver --reply-channel "$OPENCLAW_REPLY_CHANNEL" --session-id "$OPENCLAW_SESSION_ID" -m "job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"' \
  -- long-running-command --flag value
```

Example explicit agent re-entry:

```bash
agent-exec run \
  --notify-command 'openclaw agent --deliver --reply-channel "$OPENCLAW_REPLY_CHANNEL" --session-id "$OPENCLAW_SESSION_ID" -m "job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"' \
  -- long-running-command --flag value
```

Example updating notification after the job has already started:

```bash
JOB=$(agent-exec run --snapshot-after 0 -- long-running-command --flag value | jq -r .job_id)

agent-exec notify set "$JOB" \
  --command 'openclaw agent --deliver --reply-channel "$OPENCLAW_REPLY_CHANNEL" --session-id "$OPENCLAW_SESSION_ID" -m "job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"'
```

Use this pattern when the launcher learns the target session only after the job is already running.

Prefer `openclaw agent --deliver --reply-channel ... --session-id ... -m ...` when the completion event should wake an existing agent session and let that session decide how to continue.

Do not send the full event JSON unless the receiver truly needs it. In most same-host cases, the agent can inspect the event and the job itself once it knows the job id and event path.

## Good patterns

- Keep the one-liner idempotent so duplicate delivery attempts do not create harmful side effects.
- Prefer `AGENT_EXEC_EVENT_PATH` over temp files when the downstream command accepts a file path.
- Use absolute paths when PATH differences may exist inside the sink process.
- Preserve the job id in outbound messages so later `status`, `tail`, or `wait` calls can reference the same job.
- Check `completion_event.json.delivery_results` when downstream delivery reliability matters.

## Common mistakes

- Letting the one-liner grow into a mini program with several branches and fragile quoting.
- Hardcoding the wrong chat id, session id, or delivery mode.
- Assuming notification success changes the main job state; it does not.
- Running heavy retry logic inside the command sink instead of a separate worker.
