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
