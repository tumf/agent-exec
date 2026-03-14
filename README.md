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
| `--notify-command <COMMAND>` | — | Run a shell command when the job finishes; event JSON is sent on stdin |
| `--notify-file <PATH>` | — | Append a `job.finished` event as NDJSON |
| `--config <PATH>` | XDG default | Load shell wrapper config from a specific `config.toml` |
| `--shell-wrapper <PROG FLAGS>` | platform default | Override shell wrapper for this invocation (e.g. `"bash -lc"`) |

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

## Configuration

`agent-exec` reads an optional `config.toml` to configure the shell wrapper used for command-string execution.

### Config file location

- `$XDG_CONFIG_HOME/agent-exec/config.toml` (defaults to `~/.config/agent-exec/config.toml`)

### `config.toml` format

```toml
[shell]
unix    = ["sh", "-lc"]   # used on Unix-like platforms
windows = ["cmd", "/C"]   # used on Windows
```

Both keys are optional. Absent values fall back to the built-in platform default (`sh -lc` / `cmd /C`).

### Shell wrapper precedence

1. `--shell-wrapper <PROG FLAGS>` CLI flag (highest priority)
2. `--config <PATH>` explicit config file
3. Default XDG config file (`~/.config/agent-exec/config.toml`)
4. Built-in platform default (lowest priority)

The configured wrapper applies to **both** `run` command-string execution and `--notify-command` delivery so the two execution paths stay consistent.

### Override per invocation

```bash
agent-exec run --shell-wrapper "bash -lc" -- my_script.sh
```

### Use a custom config file

```bash
agent-exec run --config /path/to/config.toml -- my_script.sh
```

## Job Finished Events

When `run` is called with `--notify-command` or `--notify-file`, `agent-exec` emits a `job.finished` event after the job reaches a terminal state.

- `--notify-command` accepts a shell command string, executes it via the configured shell wrapper (default: `sh -lc` on Unix, `cmd /C` on Windows), and writes the event JSON to stdin.
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
  --notify-command 'cat > /tmp/agent-exec-event.json' \
  -- echo hello
```

### OpenClaw examples

#### Notify a Telegram chat directly

Pass a plain shell command string to `--notify-command`. The command runs via the configured shell wrapper (default: `sh -lc`) and has access to the event JSON on stdin and the `AGENT_EXEC_EVENT_PATH` environment variable.

```bash
agent-exec run \
  --notify-command 'openclaw message send --chat telegram:deployments --text "job $(jq -r .job_id "$AGENT_EXEC_EVENT_PATH") finished: state=$(jq -r .state "$AGENT_EXEC_EVENT_PATH")"' \
  -- long-running-command --flag value
```

For repeated use, a checked-in helper script is easier to review and maintain than a long inline command.

#### Return the event to the launching OpenClaw session

This pattern is often more flexible than sending a final user message directly from the notify command. The launching session can inspect logs, decide whether the result is meaningful, and summarize it in context. Depending on the workflow, either `openclaw message send` or `openclaw agent --session-id ... --deliver` may be the better fit.

```bash
SESSION_ID="oc_session_123"

agent-exec run \
  --notify-command "openclaw message send --session $SESSION_ID --text \"\$(jq -c . \"\$AGENT_EXEC_EVENT_PATH\")\"" \
  -- ./scripts/run-heavy-task.sh
```

With this pattern, the receiving OpenClaw session can read the event payload, inspect `stdout_log_path` or `stderr_log_path`, and decide whether to reply, retry, or trigger follow-up work.

If you want explicit agent re-entry instead of lightweight message delivery, call `openclaw agent --deliver` directly:

```bash
SESSION_ID="oc_session_123"

agent-exec run \
  --notify-command "openclaw agent --session-id $SESSION_ID --deliver \"\$(jq -c . \"\$AGENT_EXEC_EVENT_PATH\")\"" \
  -- ./scripts/run-heavy-task.sh
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

- `--notify-command` accepts a plain shell command string; no JSON encoding is needed.
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
