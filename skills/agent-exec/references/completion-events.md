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

Command sinks also receive:

- `AGENT_EXEC_EVENT_PATH`: path to persisted `completion_event.json`
- `AGENT_EXEC_JOB_ID`: job id
- `AGENT_EXEC_EVENT_TYPE`: currently `job.finished`

## Persistence and delivery

- Expect `completion_event.json` in the job directory.
- Expect `completion_event.json` to include the event plus `delivery_results` for each sink.
- Treat notification delivery as best effort; sink failures do not change the final job state.
- Inspect `delivery_results` when notification success matters.
