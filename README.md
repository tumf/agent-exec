# agent-exec

Non-interactive agent job runner. Runs commands as background jobs and returns structured JSON on stdout.

## Output Contract

- **stdout**: JSON only — every command prints exactly one JSON object
- **stderr**: Diagnostic logs (controlled by `RUST_LOG` or `-v`/`-vv` flags)

This separation lets agents parse stdout reliably without filtering log noise.

## Installation

```bash
cargo install --path .
```

## Quick Start

### Short-lived job (`run` → `wait` → `tail`)

Run a command, wait for it to finish, then read its output:

```bash
# 1. Start the job (returns immediately with a job_id)
JOB=$(agent-exec run echo "hello world" | jq -r .job_id)

# 2. Wait for completion
agent-exec wait "$JOB"

# 3. Read output
agent-exec tail "$JOB"
```

Example output of `tail`:

```json
{
  "schema_version": "0.1",
  "ok": true,
  "type": "tail",
  "job_id": "01J...",
  "stdout_tail": "hello world",
  "stderr_tail": "",
  "truncated": false
}
```

### Long-running job (`run` → `status` → `tail`)

Start a background job, poll its status, then read its output:

```bash
# 1. Start the job (returns immediately with a job_id)
JOB=$(agent-exec run sleep 30 | jq -r .job_id)

# 2. Check status
agent-exec status "$JOB"

# 3. Stream output tail
agent-exec tail "$JOB"

# 4. Wait for completion
agent-exec wait "$JOB"
```

### Timeout and force-kill

Run a job with a timeout; SIGTERM after 5 s, SIGKILL after 2 s more:

```bash
agent-exec run \
  --timeout 5000 \
  --kill-after 2000 \
  sleep 60
```

## Commands

### `run` — start a background job

```bash
agent-exec run [OPTIONS] <COMMAND>...
```

Key options:

| Flag | Default | Description |
|------|---------|-------------|
| `--snapshot-after <ms>` | 10000 | Wait N ms before returning (0 = return immediately) |
| `--timeout <ms>` | 0 (none) | Kill job after N ms |
| `--kill-after <ms>` | 0 | ms after SIGTERM to send SIGKILL |
| `--tail-lines <N>` | 50 | Lines of output captured in the snapshot |
| `--cwd <dir>` | inherited | Working directory |
| `--env KEY=VALUE` | — | Set environment variable (repeatable) |
| `--mask KEY` | — | Redact secret values from JSON output (repeatable) |
| `--wait` | false | Block until the job reaches a terminal state |
| `--wait-poll-ms <ms>` | 200 | Poll interval used with `--wait` |
| `--notify-command <JSON_ARGV>` | — | Run a command when the job finishes; event JSON is sent on stdin |
| `--notify-file <PATH>` | — | Append a `job.finished` event as NDJSON |

### `status` — get job state

```bash
agent-exec status <JOB_ID>
```

Returns `running`, `exited`, `killed`, or `failed`, plus `exit_code` when finished.

### `tail` — read output

```bash
agent-exec tail [--tail-lines N] <JOB_ID>
```

Returns the last N lines of stdout and stderr.

### `wait` — block until done

```bash
agent-exec wait [--timeout-ms N] [--poll-ms N] <JOB_ID>
```

Polls until the job finishes or the timeout elapses.

### `kill` — send signal

```bash
agent-exec kill [--signal TERM|INT|KILL] <JOB_ID>
```

### `list` — list jobs

```bash
agent-exec list [--state running|exited|killed|failed] [--limit N]
```

## Job Finished Events

When `run` is called with `--notify-command` or `--notify-file`, `agent-exec` emits a `job.finished` event after the job reaches a terminal state.

- `--notify-command` runs the provided argv without a shell and writes the event JSON to stdin.
- `--notify-file` appends the event as a single NDJSON line.
- `completion_event.json` is also written in the job directory with the event plus sink delivery results.
- Notification delivery is best effort; sink failures do not change the main job state.
- When delivery success matters, inspect `completion_event.json.delivery_results`.

Choose the sink based on the next consumer:

- Use `--notify-command` for small, direct reactions such as posting to chat or forwarding the event back to the launching OpenClaw session with either `openclaw message send` or `openclaw agent --session-id ... --deliver`.
- Use `--notify-file` when you want a durable queue-like handoff to a separate worker that can retry or fan out.
- Prefer checked-in helper scripts over large inline shell or Python snippets.

Example:

```bash
agent-exec run \
  --wait \
  --notify-file /tmp/agent-exec-events.ndjson \
  -- echo hello
```

Command sink example:

```bash
agent-exec run \
  --wait \
  --notify-command '["/bin/sh","-c","cat > /tmp/agent-exec-event.json"]' \
  -- echo hello
```

### OpenClaw examples

#### Notify a Telegram chat directly

Use a small checked-in helper so the notify command stays easy to review. `openclaw message send` can be appropriate for either user-facing notifications or lightweight delivery back to an agent-facing session.

```bash
agent-exec run \
  --notify-command '["./scripts/notify-telegram.sh"]' \
  -- long-running-command --flag value
```

Example helper shape:

```bash
#!/usr/bin/env bash
set -euo pipefail

tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cat > "$tmp"

job_id=$(jq -r '.job_id' "$tmp")
state=$(jq -r '.state' "$tmp")
exit_code=$(jq -r '.exit_code // "n/a"' "$tmp")

openclaw message send \
  --chat telegram:deployments \
  --text "job ${job_id} finished with state=${state} exit_code=${exit_code}"
```

#### Return the event to the launching OpenClaw session

This pattern is often more flexible than sending a final user message directly from the notify command. The launching session can inspect logs, decide whether the result is meaningful, and summarize it in context. Depending on the workflow, either `openclaw message send` or `openclaw agent --session-id ... --deliver` may be the better fit.

```bash
SESSION_ID="oc_session_123"

agent-exec run \
  --notify-command "[\"./scripts/return-to-openclaw-session.sh\",\"${SESSION_ID}\"]" \
  -- ./scripts/run-heavy-task.sh
```

Example helper shape using `openclaw message send`:

```bash
#!/usr/bin/env bash
set -euo pipefail

session_id="$1"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cat > "$tmp"

openclaw message send \
  --session "$session_id" \
  --text "$(jq -c . "$tmp")"
```

With this pattern, the receiving OpenClaw session can read the event payload, inspect `stdout_log_path` or `stderr_log_path`, and decide whether to reply, retry, or trigger follow-up work.

If you want explicit agent re-entry instead of lightweight message delivery, use a helper like this:

```bash
#!/usr/bin/env bash
set -euo pipefail

session_id="$1"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cat > "$tmp"

openclaw agent \
  --session-id "$session_id" \
  --deliver "$(jq -c . "$tmp")"
```

In practice, both `message send` and `agent --deliver` can target either a user-facing or agent-facing flow; pick the one that matches the downstream behavior you want.

#### Durable file-based worker

Use `--notify-file` when you want retries or fanout outside the main job lifecycle:

```bash
agent-exec run \
  --notify-file /var/lib/agent-exec/events.ndjson \
  -- ./scripts/run-heavy-task.sh
```

A separate worker can tail or batch-process the NDJSON file, retry failed downstream sends, and route events to chat, webhooks, or OpenClaw sessions without coupling that logic to the main job completion path.

### Operational guidance

- `--notify-command` must be a JSON argv array, not a shell string.
- Keep notify commands small, fast, and idempotent.
- Common sink failures include quoting mistakes, PATH or env mismatches, downstream non-zero exits, and wrong chat, session, or delivery-mode targets.
- If you need heavier orchestration, let the notify sink hand off to a checked-in helper or durable worker.

For command sinks, the event JSON is written to stdin and these environment variables are set:

- `AGENT_EXEC_EVENT_PATH`: path to the persisted `completion_event.json`
- `AGENT_EXEC_JOB_ID`: finished job id
- `AGENT_EXEC_EVENT_TYPE`: currently `job.finished`

Example `job.finished` payload:

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

If the job is killed by a signal, `state` becomes `killed`, `exit_code` may be absent, and `signal` is populated when available.

## Logging

Logs go to **stderr** only. Use `-v` / `-vv` or `RUST_LOG`:

```bash
RUST_LOG=debug agent-exec run echo hello
agent-exec -v run echo hello
```

## Development

```bash
cargo build
cargo test --all
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```
