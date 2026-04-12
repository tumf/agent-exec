# agent-exec

Non-interactive agent job runner. Runs commands as background jobs and returns structured JSON on stdout.

## Output Contract

- **stdout**: JSON by default — every command prints exactly one JSON object; pass `--yaml` to get YAML instead
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

## Two-step job lifecycle (create / start)

In addition to the immediate `run` path, `agent-exec` supports a two-step
lifecycle where you define a job first and start it later.

```bash
# Step 1 — define the job (no process is spawned)
JOB=$(agent-exec create -- echo "deferred hello" | jq -r .job_id)

# Step 2 — launch the job when ready
agent-exec start "$JOB"
```

- `create` persists the command, environment, timeouts, and notification
  settings to `meta.json` and writes `state.json` with `state="created"`.
  It returns `type="create"` and the `job_id`.
- `start` reads the persisted definition and spawns the supervisor.
  It returns launch metadata only; use `wait`/`tail` for observation.
- `run` remains available as the convenience path for immediate execution (also launch metadata only).

### Persisted environment

`--env KEY=VALUE` values provided to `create` are stored in `meta.json` as
durable (non-secret) configuration and applied when `start` is called.
`--env-file FILE` stores the file path; the file is re-read at `start` time.

### State transitions

| State | Meaning |
|-------|---------|
| `created` | Job definition persisted, no process running |
| `running` | Supervisor and child process active |
| `exited` | Process exited normally |
| `killed` | Process terminated by signal |
| `failed` | Supervisor-level failure |

`kill` rejects `created` jobs (no process to signal).
`wait` polls through `created` and `running` until a terminal state.
`list --state created` filters to not-yet-started jobs.

## Global Options

| Flag | Default | Description |
|------|---------|-------------|
| `--root <PATH>` | XDG default | Override the jobs root directory for all subcommands. Precedence: `--root` > `AGENT_EXEC_ROOT` > `$XDG_DATA_HOME/agent-exec/jobs` > platform default. |
| `--yaml` | false | Output responses as YAML instead of JSON (applies to all subcommands). |
| `-v` / `-vv` | warn | Increase log verbosity (logs go to stderr). |

The `--root` flag is a **global** option that applies to all job-store subcommands (`run`, `status`, `tail`, `wait`, `kill`, `list`, `gc`). The preferred placement is before the subcommand name:

```bash
agent-exec --root /tmp/jobs run echo hello
agent-exec --root /tmp/jobs status <JOB_ID>
agent-exec --root /tmp/jobs list
agent-exec --root /tmp/jobs gc --dry-run
```

For backward compatibility, `--root` is also accepted after the subcommand name (both forms are equivalent):

```bash
agent-exec run --root /tmp/jobs echo hello
agent-exec status --root /tmp/jobs <JOB_ID>
```

## Commands

### `create` — define a job without starting it

```bash
agent-exec create [OPTIONS] -- <COMMAND>...
```

Persists the job definition. Accepts the same definition-time options as `run`
(command, `--cwd`, `--env`, `--env-file`, `--mask`, `--stdin`, `--stdin-file`,
`--timeout`, `--kill-after`, `--progress-every`, `--notify-command`,
`--notify-file`, `--shell-wrapper`).
Does **not** accept observation options (`--tail-lines`, `--max-bytes`, `--wait`).

`--stdin` / `--stdin-file` are materialized into `<job-dir>/stdin.bin` during
`create`. Later `start` reuses the persisted `meta.json.stdin_file` value and
does not require additional stdin flags.

Returns `type="create"`, `state="created"`, `job_id`, `stdout_log_path`,
and `stderr_log_path`.

### `start` — launch a previously created job

```bash
agent-exec start [OPTIONS] <JOB_ID>
```

Launches the job whose definition was persisted by `create`.

`start` is launch-only and accepts only the job identifier (plus global flags like `--root`/`--yaml`).
Observation is intentionally separated: use `wait` for completion state and `tail` for output.

Returns `type="start"` launch metadata (`job_id`, `state`, `stdout_log_path`, `stderr_log_path`, `elapsed_ms`).
Only jobs in `created` state can be started; any other state returns `error.code="invalid_state"`.

### `run` — start a background job

```bash
agent-exec run [OPTIONS] <COMMAND>...
```

Key options:

| Flag | Default | Description |
|------|---------|-------------|
| `--timeout <seconds>` | 0 (none) | Kill job after N seconds |
| `--kill-after <seconds>` | 0 | Seconds after SIGTERM to send SIGKILL |
| `--cwd <dir>` | inherited | Working directory |
| `--env KEY=VALUE` | — | Set environment variable (repeatable) |
| `--mask KEY` | — | Redact secret values from JSON output (repeatable) |
| `--stdin <VALUE>` | — | Provide job stdin content directly. Use `--stdin -` for pipe/heredoc/redirect input. |
| `--stdin-file <PATH>` | — | Copy file contents into job-local `stdin.bin` and use it as child stdin. |
| `--tag <TAG>` | — | Assign a user-defined tag to the job (repeatable; duplicates deduplicated) |
| `--notify-command <COMMAND>` | — | Run a shell command when the job finishes; event JSON is sent on stdin |
| `--notify-file <PATH>` | — | Append a `job.finished` event as NDJSON |
| `--config <PATH>` | XDG default | Load shell wrapper config from a specific `config.toml` |
| `--shell-wrapper <PROG FLAGS>` | platform default | Override shell wrapper for this invocation (e.g. `"bash -lc"`) |

`--stdin` and `--stdin-file` are mutually exclusive. When `--stdin -` is used,
`agent-exec` requires non-interactive stdin; if caller stdin is a tty it fails
fast with `error.code = "stdin_required"`.

```bash
# Pipe stdin into the job
printf 'abc' | agent-exec run --stdin - -- cat

# Heredoc stdin
agent-exec run --stdin - -- cat <<'EOF'
line1
line2
EOF

# Inline stdin (no implicit newline added)
agent-exec run --stdin "abc" -- cat

# File-backed stdin (materialized to <job-dir>/stdin.bin)
agent-exec run --stdin-file ./input.txt -- cat
```

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
agent-exec wait [--until SECONDS] [--poll SECONDS] [--forever] <JOB_ID>
```

Polls until the job reaches a terminal state or the wait deadline elapses.
`--until` is a client-side wait deadline and does not stop the underlying job.
Use `run --timeout` when you need a process runtime limit.

### `kill` — send signal

```bash
agent-exec kill [--signal TERM|INT|KILL] <JOB_ID>
```

### `list` — list jobs

```bash
agent-exec list [--state created|running|exited|killed|failed] [--limit N] [--tag PATTERN]...
```

By default only jobs from the current working directory are shown. Use `--all` to show jobs from all directories.

Tag filtering with `--tag` applies logical AND across all patterns. Two pattern forms are supported:

- **Exact**: `--tag aaa` matches only jobs that have the tag `aaa`.
- **Namespace prefix**: `--tag hoge.*` matches jobs with any tag in the `hoge` namespace (e.g. `hoge.sub`, `hoge.sub.deep`).

```bash
# Show jobs tagged with "ci"
agent-exec list --all --tag ci

# Show jobs in the "project.build" namespace across all directories
agent-exec list --all --tag project.build.*

# Combine: jobs tagged with both "ci" AND "release" in the current cwd
agent-exec list --tag ci --tag release
```

### `tag set` — replace job tags

```bash
agent-exec tag set <JOB_ID> [--tag TAG]...
```

Replaces all tags on an existing job with the specified list. Duplicates are deduplicated preserving first-seen order. Omit all `--tag` flags to clear tags.

```bash
# Assign tags at creation time
agent-exec run --tag project.build --tag ci -- make build

# Replace tags on an existing job
agent-exec tag set 01J9ABC123 --tag project.release --tag approved

# Clear all tags
agent-exec tag set 01J9ABC123
```

**Tag format**: dot-separated segments of alphanumeric characters and hyphens (e.g. `ci`, `project.build`, `hoge-fuga.v2`). The `.*` suffix is reserved for list filter patterns and cannot be used as a stored tag.

### `notify set` — update notification configuration

```bash
agent-exec notify set <JOB_ID> [--command <COMMAND>] \
  [--output-pattern <PATTERN>] [--output-match-type contains|regex] \
  [--output-stream stdout|stderr|either] \
  [--output-command <COMMAND>] [--output-file <PATH>]
```

Updates the persisted notification configuration for an existing job. This is a **metadata-only** operation: it rewrites `meta.json` and never executes sinks immediately, even when the target job is already in a terminal state.

**Completion notification flags:**

| Flag | Description |
|------|-------------|
| `--command <COMMAND>` | Shell command string for the `job.finished` command sink. |
| `--root <PATH>` | Override the jobs root directory. |

**Output-match notification flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--output-pattern <PATTERN>` | — | Pattern to match against newly observed stdout/stderr lines. Required to enable output-match notifications. |
| `--output-match-type <TYPE>` | `contains` | `contains` for substring matching; `regex` for Rust regex syntax. |
| `--output-stream <STREAM>` | `either` | `stdout`, `stderr`, or `either` — which stream is eligible for matching. |
| `--output-command <COMMAND>` | — | Shell command string executed on every match; event JSON is sent on stdin. |
| `--output-file <PATH>` | — | File that receives one NDJSON `job.output.matched` event per match. |

**Behavior**

- All flags are optional; unspecified fields are preserved from the existing configuration.
- `--command` replaces the existing `notify_command`; `notify_file` is always preserved.
- Output-match configuration is stored under `meta.json.notification.on_output_match`.
- Once saved, output-match settings apply only to **future** lines observed by the running supervisor — prior output is never replayed.
- Calling `notify set` on a terminal job succeeds without executing any sink.
- A missing job returns a JSON error with `error.code = "job_not_found"`.

**Example — completion notification**

```bash
JOB=$(agent-exec run -- sleep 5 | jq -r .job_id)
agent-exec notify set "$JOB" --command 'cat > /tmp/event.json'
```

**Example — output-match notification**

```bash
# Run a job that may print error lines.
JOB=$(agent-exec run -- sh -c 'sleep 1; echo ERROR foo' | jq -r .job_id)

# Configure output-match: fire on every line containing "ERROR".
agent-exec notify set "$JOB" \
  --output-pattern 'ERROR' \
  --output-command 'cat >> /tmp/matches.ndjson'

# Or use a regex pattern targeting only stderr:
agent-exec notify set "$JOB" \
  --output-pattern '^ERR' \
  --output-match-type regex \
  --output-stream stderr \
  --output-file /tmp/stderr_matches.ndjson
```
### `gc` — garbage collect old job data

```bash
agent-exec [--root <PATH>] gc [--older-than <DURATION>] [--dry-run]
```

Deletes job directories under the root whose terminal state (`exited`, `killed`, or `failed`) is older than the retention window. Running jobs are never touched.

| Flag | Default | Description |
|------|---------|-------------|
| `--older-than <DURATION>` | `30d` | Retention window: jobs older than this are eligible for deletion. Supports `30d`, `24h`, `60m`, `3600s`. |
| `--dry-run` | false | Report candidates without deleting anything. |

**Retention semantics**

- The GC timestamp used for age evaluation is `finished_at` when present, falling back to `updated_at`.
- Jobs where both timestamps are absent are skipped safely.
- `running` jobs are never deleted regardless of age.

**Examples**

```bash
# Preview what would be deleted (30-day default window).
agent-exec gc --dry-run

# Preview with a custom 7-day window.
agent-exec gc --older-than 7d --dry-run

# Delete jobs older than 7 days.
agent-exec gc --older-than 7d

# Operate on a specific jobs root directory.
agent-exec --root /tmp/jobs gc --older-than 7d
```

**JSON response fields**

| Field | Type | Description |
|-------|------|-------------|
| `root` | string | Resolved jobs root path |
| `dry_run` | bool | Whether this was a preview-only run |
| `older_than` | string | Effective retention window (e.g. `"30d"`) |
| `older_than_source` | string | `"default"` or `"flag"` |
| `deleted` | number | Count of directories actually deleted |
| `skipped` | number | Count of directories skipped |
| `freed_bytes` | number | Bytes freed (or would be freed in dry-run) |
| `jobs` | array | Per-job details: `job_id`, `state`, `action`, `reason`, `bytes` |

The `action` field in each `jobs` entry is one of:
- `"deleted"` — directory was removed
- `"would_delete"` — would be removed in a real run (dry-run only)
- `"skipped"` — preserved with an explanation in `reason`

## delete

Explicitly remove one or all finished job directories. Unlike `gc`, which uses
age-based retention across the whole jobs root, `delete` is operator-driven:
remove one known job immediately, or clear finished jobs belonging to the
current working directory.

```
agent-exec delete <JOB_ID>
agent-exec delete --all [--dry-run]
```

**State rules**

- `delete <JOB_ID>` — removes jobs in state `created`, `exited`, `killed`, or
  `failed`. Returns an error for `running` jobs (the job directory is preserved).
- `delete --all` — removes only terminal jobs (`exited`, `killed`, `failed`)
  whose persisted `meta.json.cwd` matches the caller's current working
  directory. Jobs in `created` or `running` state are skipped and reported in
  the response.

**Examples**

```bash
# Remove a specific finished job.
agent-exec delete 01JA1B2C3D4E5F6G7H8I9J0K1L

# Preview which jobs would be removed from the current directory.
agent-exec delete --dry-run --all

# Remove all terminal jobs from the current directory.
agent-exec delete --all

# Operate on a specific jobs root.
agent-exec --root /tmp/jobs delete --all
```

**JSON response fields**

| Field | Type | Description |
|-------|------|-------------|
| `root` | string | Resolved jobs root path |
| `dry_run` | bool | Whether this was a preview-only run |
| `deleted` | number | Count of directories actually deleted (0 when `dry_run=true`) |
| `skipped` | number | Count of in-scope directories that were not deleted |
| `jobs` | array | Per-job details: `job_id`, `state`, `action`, `reason` |

The `action` field in each `jobs` entry is one of:
- `"deleted"` — directory was removed
- `"would_delete"` — would be removed in a real run (dry-run only)
- `"skipped"` — preserved with an explanation in `reason` (e.g. `"running"`, `"created"`)

**Difference between `delete` and `gc`**

| | `delete` | `gc` |
|--|----------|------|
| Scope | Single job or cwd-scoped finished jobs | Entire jobs root |
| Trigger | Explicit operator action | Age-based retention policy |
| Running jobs | Always rejected / skipped | Always skipped |
| Dry-run | `--dry-run` flag | `--dry-run` flag |

## serve — HTTP API server

`agent-exec serve` starts a REST API server that exposes job operations over HTTP.
This allows Flowise, curl, or any HTTP client to launch and monitor jobs without
needing direct access to the CLI.

```bash
agent-exec serve [--bind HOST:PORT] [--port PORT]
```

**Default bind address**: `127.0.0.1:19263` (localhost only, not exposed externally).

### Network security note

The server performs **no authentication**. Access is controlled by the bind address:
- `127.0.0.1` (default): only reachable from the same host — safe for local use.
- `0.0.0.0`: reachable from all network interfaces — **requires a firewall or reverse proxy** to restrict access.

### Endpoints

| Method | Path            | CLI equivalent | Description                                      |
|--------|-----------------|----------------|--------------------------------------------------|
| GET    | /health         | —              | Health check. Returns `{"ok":true}`              |
| POST   | /exec           | `run`          | Launch a job; returns `job_id`                   |
| GET    | /status/{id}    | `status`       | Job status                                       |
| GET    | /tail/{id}      | `tail`         | stdout/stderr log tail                           |
| GET    | /wait/{id}      | `wait`         | Block until job reaches a terminal state         |
| POST   | /kill/{id}      | `kill`         | Send SIGTERM to the job                          |

All responses include `schema_version`, `ok`, and `type` fields matching the CLI schema.

### POST /exec request body

```json
{
  "command": ["bash", "-c", "echo hello"],
  "cwd": "/tmp",
  "env": {"FOO": "bar"},
  "timeout_ms": 30000
}
```

Only `command` is required. Returns the same `RunData` as the `run` CLI command.

### Flowise / Docker example

From a Flowise container, use `host.docker.internal` to reach the agent-exec server
running on the host:

```
POST http://host.docker.internal:19263/exec
{"command": ["my-agent-script"]}
```

Then poll `GET http://host.docker.internal:19263/wait/{job_id}` until the job finishes.

To allow container access, start the server with `--bind 0.0.0.0:19263` and ensure
your firewall does **not** expose port 19263 to the public internet.

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

### Command launch modes (Unix)

`agent-exec run` supports two launch modes, selected by the number of arguments after `--`:

| Mode | Example | Behaviour |
|------|---------|-----------|
| **Shell-string** | `agent-exec run -- "echo hi && ls"` | Single argument is passed as-is to the shell wrapper. Shell operators (`&&`, pipes, etc.) are preserved. The wrapper process is the workload boundary. |
| **Argv** | `agent-exec run -- cflx run` | Two or more arguments trigger an `exec "$@"` handoff. The shell wrapper runs briefly for login-shell environment initialisation, then replaces itself with the target workload. The observed child PID and lifecycle align with the intended command, not the shell. |

The `exec` handoff means that for argv-mode invocations, completion tracking aligns with the target workload rather than the shell wrapper, which resolves lingering-shell issues when the target replaces the wrapper process.

The configured wrapper applies to **both** `run` command-string execution and `--notify-command` delivery. Notify delivery always uses shell-string mode regardless of how the job was launched.

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

- Use `--notify-command` for small, direct reactions such as forwarding the event back to the launching OpenClaw session with `openclaw agent --deliver --reply-channel ... --session-id ... -m ...`.
- Use `--notify-file` when you want a durable queue-like handoff to a separate worker that can retry or fan out.
- Prefer a compact one-liner for agent-authored OpenClaw callbacks, and prefer `AGENT_EXEC_EVENT_PATH` over parsing stdin when the downstream command accepts a file.

Example:

```bash
JOB=$(agent-exec run --notify-file /tmp/agent-exec-events.ndjson -- echo hello | jq -r .job_id)
agent-exec wait "$JOB"
agent-exec tail "$JOB"
```

Command sink example:

```bash
JOB=$(agent-exec run --notify-command 'cat > /tmp/agent-exec-event.json' -- echo hello | jq -r .job_id)
agent-exec wait "$JOB"
agent-exec tail "$JOB"
```

### OpenClaw examples

#### Return the event to the launching OpenClaw session

This pattern is often more flexible than sending a final user message directly from the notify command. The launching session can inspect logs, decide whether the result is meaningful, and summarize it in context. In same-host agent-to-agent flows, `job_id` plus `event_path` is a good default.

Call `openclaw agent --deliver` with the reply channel and session id directly:

```bash
SESSION_ID="01bb09d5-6485-4a50-8d3b-3f6e80c61f9c"
REPLY_CHANNEL="telegram"

agent-exec run \
  --notify-command "openclaw agent --deliver --reply-channel $REPLY_CHANNEL --session-id $SESSION_ID -m \"job_id=\$AGENT_EXEC_JOB_ID event_path=\$AGENT_EXEC_EVENT_PATH\"" \
  -- ./scripts/run-heavy-task.sh
```

With this pattern, the receiving OpenClaw session can open the persisted event file immediately and still keep the job id for follow-up commands.

Prefer sending `job_id` and `event_path` instead of the full JSON blob when the receiver can access the same filesystem.

#### Attach or replace the callback later with `notify set`

Use `notify set` when the job is already running and you only learn the OpenClaw destination afterward.

```bash
JOB=$(agent-exec run -- ./scripts/run-heavy-task.sh | jq -r .job_id)
SESSION_ID="01bb09d5-6485-4a50-8d3b-3f6e80c61f9c"
REPLY_CHANNEL="telegram"

agent-exec notify set "$JOB" \
  --command "openclaw agent --deliver --reply-channel $REPLY_CHANNEL --session-id $SESSION_ID -m \"job_id=\$AGENT_EXEC_JOB_ID event_path=\$AGENT_EXEC_EVENT_PATH\""
```

`notify set` is metadata-only: it updates the stored callback for future completion delivery and does not execute the sink immediately.

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
- Prefer `AGENT_EXEC_EVENT_PATH` when the downstream command already knows how to read a file.
- Common sink failures include quoting mistakes, PATH or env mismatches, downstream non-zero exits, and wrong chat, session, or delivery-mode targets.
- If you need heavier orchestration, let the notify sink hand off to a checked-in helper or durable worker.

For command sinks, the event JSON is written to stdin and these environment variables are set:

- `AGENT_EXEC_EVENT_PATH`: path to the persisted event file (`completion_event.json` for `job.finished`, `notification_events.ndjson` for `job.output.matched`)
- `AGENT_EXEC_JOB_ID`: job id
- `AGENT_EXEC_EVENT_TYPE`: `job.finished` or `job.output.matched`

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

## Output-Match Events

When a job has output-match notification configuration (set via `notify set --output-pattern`), the running supervisor evaluates each newly observed stdout/stderr line and emits a `job.output.matched` event for every line that matches.

**Key properties:**

- Delivery fires on **every matching line**, not once per job.
- Only **future** lines are eligible — output produced before `notify set` was called is never replayed.
- Sink failures are recorded in `notification_events.ndjson` and do not affect the job lifecycle state.
- Matching uses either `contains` (substring) or `regex` (Rust regex syntax) as configured by `--output-match-type`.
- Stream selection (`--output-stream`) restricts matching to `stdout`, `stderr`, or `either`.

Example `job.output.matched` payload:

```json
{
  "schema_version": "0.1",
  "event_type": "job.output.matched",
  "job_id": "01J...",
  "pattern": "ERROR",
  "match_type": "contains",
  "stream": "stdout",
  "line": "ERROR: connection refused",
  "stdout_log_path": "/jobs/01J.../stdout.log",
  "stderr_log_path": "/jobs/01J.../stderr.log"
}
```

Delivery records for output-match events are appended to `notification_events.ndjson` in the job directory (one JSON object per line). The `completion_event.json` file retains only `job.finished` delivery results.

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
