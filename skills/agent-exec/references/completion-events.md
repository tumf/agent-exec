# Completion Events

## Event shape

When notification sinks are configured, expect a `job.finished` payload after the job reaches terminal state.

```json
{
  "schema_version": "0.1",
  "event_type": "job.finished",
  "job_id": "01J...",
  "state": "exited",
  "command": ["echo", "hello"],
  "cwd": "/path/to/cwd",
  "started_at": "2026-03-15T12:00:00Z",
  "finished_at": "2026-03-15T12:00:00Z",
  "duration_ms": 12,
  "exit_code": 0,
  "stdout_log_path": "/jobs/01J.../stdout.log",
  "stderr_log_path": "/jobs/01J.../stderr.log"
}
```

Possible fields:

- identity: `schema_version`, `event_type`, `job_id`
- outcome: `state`, optional `exit_code`, optional `signal`
- execution context: `command`, optional `cwd`, `started_at`, `finished_at`, optional `duration_ms`
- artifacts: `stdout_log_path`, `stderr_log_path`

If the job is killed by a signal, expect `state` to become `killed`; `signal` may be present and `exit_code` may be absent.

## Sinks

Use these `run` options:

- `--notify-command <JSON_ARGV>`: spawn a command without a shell and write the event JSON to stdin
- `--notify-file <PATH>`: append one NDJSON line per completed job

### Choosing a sink

- Use `--notify-command` for immediate, low-latency reactions such as posting to chat, invoking a webhook helper, or returning the event to the launching OpenClaw session via either `openclaw message send` or `openclaw agent --session-id ... --deliver`, depending on the workflow.
- Use `--notify-file` when a durable worker should process events later, retries are important, or multiple downstream consumers need the same event stream.
- Prefer checked-in helper scripts over large inline shell or Python snippets. Small wrappers are easier to quote correctly, review, and reuse.

Command sinks also receive:

- `AGENT_EXEC_EVENT_PATH`: path to persisted `completion_event.json`
- `AGENT_EXEC_JOB_ID`: job id
- `AGENT_EXEC_EVENT_TYPE`: currently `job.finished`

## Persistence and delivery

- Expect `completion_event.json` in the job directory.
- Expect `completion_event.json` to include the event plus `delivery_results` for each sink.
- Treat notification delivery as best effort; sink failures do not change the final job state.
- Inspect `delivery_results` when notification success matters.

### Best practices

- Keep command sinks small, fast, and idempotent.
- Use stdin for the full event JSON and the environment variables for cheap routing metadata.
- Treat command sinks as a trigger, not a workflow engine; move retries, fanout, and heavier logic into a checked-in helper or a separate worker.

### Common failure modes

- Wrong quoting when passing `--notify-command`; it must be a JSON argv array, not a shell pipeline string.
- PATH or environment mismatch inside the sink process; use absolute paths or wrapper scripts when possible.
- Downstream command exits non-zero even though the main job succeeded.
- Wrong reply target, chat id, session id, or delivery mode in the notify helper.
- Notify helper assumes delivery success without checking `completion_event.json.delivery_results` afterward.

### OpenClaw-oriented patterns

- Notify a chat or session directly: a helper reads the event from stdin and uses `openclaw message send` when you want a lightweight delivery path for either a user or an agent.
- Return the event to the launching OpenClaw session: a helper forwards the event back to the original session with either `openclaw message send` or `openclaw agent --session-id ... --deliver`; choose based on whether you want simple delivery or explicit agent re-entry.
- Durable file worker: append events with `--notify-file` and let a separate process handle retries, fanout, or rate-limited APIs.
