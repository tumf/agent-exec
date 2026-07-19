# agent-exec

A durable command runner for AI agents: start a command now, observe it briefly, then retrieve or control the job later.

Agent harnesses often need to run tests, builds, and deployments whose duration and output size are unknown. A synchronous subprocess can block the agent, while a detached shell command loses structured status and log discovery. `agent-exec` keeps the process running under a stable `job_id` and returns machine-readable JSON for later `status`, `tail`, `wait`, `kill`, or `restart` calls.

## Try it in 30 seconds

Prebuilt binaries are available for Linux x86_64 and macOS Apple Silicon. This example downloads the latest release, verifies its SHA-256 checksum, and installs it in `~/.local/bin`:

```bash
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) TARGET=x86_64-unknown-linux-gnu ;;
  Darwin-arm64) TARGET=aarch64-apple-darwin ;;
  *) echo "No release binary for this platform" >&2; exit 1 ;;
esac

VERSION=$(curl -fsSL https://api.github.com/repos/tumf/agent-exec/releases/latest | sed -n 's/.*"tag_name": "v\([^"]*\)".*/\1/p')
ARCHIVE="agent-exec-v${VERSION}-${TARGET}.tar.gz"
curl -fLO "https://github.com/tumf/agent-exec/releases/download/v${VERSION}/${ARCHIVE}"
curl -fLO "https://github.com/tumf/agent-exec/releases/download/v${VERSION}/${ARCHIVE}.sha256"
shasum -a 256 -c "${ARCHIVE}.sha256"
mkdir -p ~/.local/bin
tar -xzf "$ARCHIVE"
install agent-exec ~/.local/bin/agent-exec
~/.local/bin/agent-exec --version
```

Start a long-running command without blocking the caller, save its `job_id`, then retrieve its status, logs, and final result:

```bash
AGENT_EXEC=~/.local/bin/agent-exec
JOB=$($AGENT_EXEC run --no-wait -- sh -c 'sleep 2; echo done' | sed -n 's/.*"job_id":"\([^"]*\)".*/\1/p')
$AGENT_EXEC status "$JOB"
$AGENT_EXEC tail "$JOB"
$AGENT_EXEC wait "$JOB"
```

`run` normally observes a job for up to 10 seconds, which catches many startup failures without blocking indefinitely. Use `--no-wait` when the caller must return immediately. Inline output is bounded; complete logs remain available at the paths in each response.

## Why not `nohup` or a plain subprocess?

| Capability | Plain subprocess | `nohup` | `agent-exec` |
|---|---:|---:|---:|
| Return without stopping the command | No | Yes | Yes |
| Stable job identifier | No | No | Yes |
| Structured status and exit result | Limited | No | Yes |
| Discoverable stdout/stderr logs | Caller-managed | Caller-managed | Yes |
| Later wait, tail, kill, and restart | Caller-managed | Caller-managed | Yes |

Pass ordinary commands as `argv` after `--`. Use an explicit shell only when the workload needs pipelines, redirects, expansion, or compound statements.

## Output Contract

Response-producing CLI commands write exactly one JSON object to `stdout` by default. `--yaml` changes those responses to YAML. Diagnostic logs go to `stderr` and are controlled by `RUST_LOG`, `-v`, and `-vv`.

This contract applies to commands such as `run`, `status`, `tail`, `list`, `gc`, and `install-skills`. It does not apply to generated shell completions, the MCP stdio protocol, the HTTP server, MCP startup configuration errors, or Clap help and version output.

## Inline Output Compression

`run`, `start`, `restart`, and `tail` include a compressed view by default while preserving the raw excerpt and byte metadata as the canonical output.

- Default mode: `route`
- CLI selection: `--compress <MODE>` or `--rtk <MODE>`
- Modes: `off`, `route`, `errors`, `tests`, `logs`, `git`, `json`, and `summary`
- Compatibility mode: `--compress off` omits the `compression` field
- Configuration: `[compression].default = "off"` or another supported mode

Precedence is `--compress` or `--rtk`, `[compression].default`, then the built-in `route` default. If compression would not make either nonempty stream smaller, the expansion guard sets `applied` to `false`, returns empty compressed streams, and records `"expansion-guard"` in `strategy`. The raw `stdout`, `stderr`, ranges, totals, encoding, and log paths remain unchanged.

## Installation

### GitHub Releases

GitHub Actions builds the Linux x86_64 archive for each `v*` release. Download its matching checksum, then verify and install:

```bash
ARCHIVE=agent-exec-v<VERSION>-x86_64-unknown-linux-gnu.tar.gz
shasum -a 256 -c "$ARCHIVE.sha256"
mkdir -p ~/.local/bin
tar -xzf "$ARCHIVE"
mv agent-exec ~/.local/bin/
agent-exec --version
agent-exec run -- echo "release-smoke"
```

macOS artifacts are built locally on the trusted `mini` host after GitHub Actions creates the release. From a checkout at the release tag:

```bash
scripts/release-macos.sh --tag v<VERSION>
scripts/release-macos.sh --tag v<VERSION> --upload
```

The first command builds, smoke-tests, packages, and checksums the native architecture archive without network mutation. The second uploads it to the existing GitHub Release. Install the resulting macOS archive with the checksum, extraction, version, and managed-command steps above.

Windows release binaries are not provided. ARM, 32-bit, musl Linux, Windows, and other unsupported targets can use crates.io or a source build.

### crates.io

A Rust toolchain can install the published package:

```bash
cargo install agent-exec --locked
agent-exec --version
agent-exec run -- echo "release-smoke"
```

### Source build

For development or unsupported targets, build from a repository checkout:

```bash
cargo install --path . --locked
```

## Shell Completions

`agent-exec` generates dynamic completion scripts for Bash, Zsh, Fish, and PowerShell.

### Candidate scope and state filters

All job ID candidates are limited to jobs whose persisted current working directory matches the caller's current working directory. Candidates come from the resolved jobs root. Entries with missing or mismatched current working directory metadata are excluded.

State filters depend on the command:

| Command | Candidate states |
|---------|------------------|
| `status`, `tail`, `restart`, `tag set`, `notify set` | All known job IDs; unreadable states may still appear |
| `start` | `created` |
| `wait` | `created`, `running` |
| `kill` | `running` |
| `delete` | `exited`, `killed`, `failed` |

Completion is advisory. Command implementations still validate the selected job and may support behavior not offered by completion.

### Bash

```bash
agent-exec completions bash >> ~/.bash_completion
source ~/.bash_completion
```

### Zsh

```bash
mkdir -p ~/.zsh/completions
agent-exec completions zsh > ~/.zsh/completions/_agent-exec
fpath=(~/.zsh/completions $fpath)
autoload -Uz compinit && compinit
```

### Fish

```bash
mkdir -p ~/.config/fish/completions
agent-exec completions fish > ~/.config/fish/completions/agent-exec.fish
```

### PowerShell

```powershell
agent-exec completions powershell | Out-String | Invoke-Expression
```

After registration, job ID completion is available at command arguments such as:

```bash
agent-exec tail <TAB>
agent-exec kill <TAB>
```

## Quick Start

### Short-lived job

The default `run` behavior observes the job for up to 10 seconds and returns inline output:

```bash
agent-exec run -- echo "hello world"
```

The following is an illustrative complete response for a successful terminal job. Paths, timestamps, and timing values vary.

```json
{
  "schema_version": "0.1",
  "ok": true,
  "type": "run",
  "job_id": "7f3a9c1e4b2d8a605e7c9f0134ab6d82",
  "state": "exited",
  "tags": [],
  "stdout_log_path": "/home/user/.local/share/agent-exec/jobs/7f3a9c1e4b2d8a605e7c9f0134ab6d82/stdout.log",
  "stderr_log_path": "/home/user/.local/share/agent-exec/jobs/7f3a9c1e4b2d8a605e7c9f0134ab6d82/stderr.log",
  "elapsed_ms": 8,
  "waited_ms": 2,
  "stdout": "hello world\n",
  "stderr": "",
  "stdout_range": [0, 12],
  "stderr_range": [0, 0],
  "stdout_total_bytes": 12,
  "stderr_total_bytes": 0,
  "encoding": "utf-8-lossy",
  "exit_code": 0,
  "finished_at": "2026-07-19T12:00:00Z",
  "duration_ms": 1,
  "compression": {
    "mode": "route",
    "applied": false,
    "detected_kind": "summary",
    "stdout": "",
    "stderr": "",
    "stdout_original_bytes": 12,
    "stderr_original_bytes": 0,
    "stdout_compressed_bytes": 0,
    "stderr_compressed_bytes": 0,
    "omitted": false,
    "strategy": ["expansion-guard"]
  }
}
```

Generated job IDs are 32-character lowercase hexadecimal strings. Commands that accept a job ID also accept an unambiguous prefix and return the canonical full job ID.

### Long-running job

Use `--no-wait` when the launch response must return immediately, then inspect the job separately:

```bash
JOB=$(agent-exec run --no-wait -- sleep 30 | jq -r .job_id)
agent-exec status "$JOB"
agent-exec tail "$JOB"
agent-exec wait "$JOB"
```

Without `--no-wait`, `run` observes for up to 10 seconds before returning.

### Runtime timeout and force kill

The following command sends `SIGTERM` after 5 seconds and `SIGKILL` 2 seconds later if necessary:

```bash
agent-exec run \
  --timeout 5 \
  --kill-after 2 \
  -- sleep 60
```

### Argv-first invocation

Pass ordinary commands as `argv` after `--`:

```bash
agent-exec run -- sleep 8
agent-exec run -- cargo test --all
agent-exec run -- npm run build
```

Use an explicit shell only when the workload requires shell syntax such as pipelines, redirects, expansion, or compound statements:

```bash
agent-exec run -- sh -lc 'sleep 8; echo done'
```

## Deferred Job Lifecycle

`create` persists a job definition without starting a process. `start` launches it later.

```bash
JOB=$(agent-exec create -- echo "deferred hello" | jq -r .job_id)
agent-exec start "$JOB"
```

- `create` writes the execution definition to `meta.json`, initializes `state.json` with `state` set to `created`, and returns `type` set to `create`.
- `start` reads the persisted definition, launches the supervisor, and observes for up to 10 seconds by default.
- `restart` reuses the job ID and definition. It terminates a running process tree before launching the replacement run.
- `run` combines definition and launch in one command.

### Persisted environment

Values passed through `create --env KEY=VALUE` are durable configuration. The real values are stored in `meta.json` for the later `start`, while the display-oriented `env_vars` metadata applies `--mask`. Use `--env-file FILE` when values should be read from a file at `start` time instead of being stored directly in the job definition.

`--mask KEY` only replaces the named `--env` value in display-oriented environment metadata and response fields. It does not redact child `stdout`, child `stderr`, persisted logs, notification payload content, or values loaded from `--env-file`. A child process that prints a secret will expose it in its output. Do not treat `--mask` as a general secret-filtering mechanism.

### State transitions

| State | Meaning |
|-------|---------|
| `created` | Definition persisted; no process started |
| `running` | Supervisor and child process active |
| `exited` | Process exited normally |
| `killed` | Process terminated by a signal |
| `failed` | Supervisor-level failure |

`kill` rejects `created` jobs because there is no process to signal. `wait` continues through `created` and `running` until a terminal state or its client-side deadline. `list --state created` selects jobs that have not started.

## Global Options

| Flag | Default | Description |
|------|---------|-------------|
| `--root <PATH>` | Platform data directory | Override the jobs root. Precedence is `--root`, `AGENT_EXEC_ROOT`, `$XDG_DATA_HOME/agent-exec/jobs`, then the platform default. |
| `--yaml` | `false` | Emit YAML instead of JSON for response-producing CLI commands. |
| `-v`, `-vv` | Warnings | Increase diagnostic verbosity on `stderr`. |

Place global options before the subcommand:

```bash
agent-exec --root /tmp/jobs run -- echo hello
agent-exec --root /tmp/jobs status <JOB_ID>
agent-exec --root /tmp/jobs list
agent-exec --root /tmp/jobs gc --dry-run
```

For backward compatibility, job-store commands also accept `--root` after the subcommand where defined:

```bash
agent-exec run --root /tmp/jobs -- echo hello
agent-exec status --root /tmp/jobs <JOB_ID>
```

Use `agent-exec --help` and `agent-exec <COMMAND> --help` for the complete current CLI surface.

## Commands

### `create`: define a job without starting it

```bash
agent-exec create [OPTIONS] -- <COMMAND> [ARGS...]
```

`create` persists execution-definition options for the command, effective working directory (`--cwd`, or the caller's current working directory), environment and inheritance, input, runtime limits, progress updates, tags, completion and output-match notifications, and shell configuration. Materialized input is limited by `--stdin-max-bytes`. It does not accept launch observation, compression, or automatic GC options. Use `agent-exec create --help` for the complete option list.

`--stdin VALUE` and `--stdin-file PATH` are mutually exclusive. Their contents are copied to `<job-directory>/stdin.bin`; later `start` reuses the persisted file reference.

The response includes `job_id`, `state`, `stdout_log_path`, and `stderr_log_path`.

### `start`: launch a created job

```bash
agent-exec start [OPTIONS] <JOB_ID>
```

Only a job in `created` state can start. By default, `start` observes for up to 10 seconds and returns the same inline stream fields as `run`. Observation controls include `--wait [true|false]`, `--until`, `--forever`, `--no-wait`, `--max-bytes`, and compression selection. Automatic GC controls are also available.

### `restart`: relaunch a job in place

```bash
agent-exec restart [OPTIONS] <JOB_ID>
```

`restart` preserves the job ID and persisted definition. If the job is `running`, it sends the signal selected by `--signal` and confirms termination before relaunching. It clears prior-run stream logs, `full.log`, and stale `completion_event.json` so subsequent observation reflects the replacement run.

`restart` supports the same inline observation, compression, and automatic GC controls as `start`.

### `run`: define and launch a job

```bash
agent-exec run [OPTIONS] -- <COMMAND> [ARGS...]
```

Common options:

| Flag | Default | Description |
|------|---------|-------------|
| `--timeout <SECONDS>` | `0` | Stop the process after this runtime; `0` disables the limit. |
| `--kill-after <SECONDS>` | `0` | Delay between `SIGTERM` and `SIGKILL` after timeout. |
| `--cwd <PATH>` | Inherited | Set the child current working directory. |
| `--env KEY=VALUE` | None | Set an environment variable; repeatable. |
| `--env-file <FILE>` | None | Load environment variables from a file; repeatable. |
| `--no-inherit-env` | `false` | Do not inherit the launcher environment. |
| `--mask <KEY>` | None | Mask the named `--env` value in display metadata; repeatable. |
| `--stdin <VALUE>` | None | Provide input directly; `--stdin -` reads noninteractive caller input. |
| `--stdin-file <PATH>` | None | Copy file content to job-local input. |
| `--stdin-max-bytes <BYTES>` | 64 MiB | Limit materialized input size. |
| `--wait [true|false]` | `true` | Enable inline observation. A bare `--wait` means `true`. |
| `--until <SECONDS>` | `10` | Bound inline observation. |
| `--forever` | `false` | Observe until the job becomes terminal. |
| `--no-wait` | `false` | Return without observation. |
| `--max-bytes <BYTES>` | `65536` | Limit the head excerpt from each stream. |
| `--tag <TAG>` | None | Assign a tag; repeatable and deduplicated. |
| `--notify-command <COMMAND>` | None | Run a shell command when the job finishes. |
| `--notify-file <PATH>` | None | Append a `job.finished` NDJSON event. |
| `--config <PATH>` | XDG default | Load a specific `config.toml`. |
| `--shell-wrapper <PROGRAM AND FLAGS>` | Config or platform default | Override the shell wrapper. |
| `--compress <MODE>` | Config or `route` | Select inline compression. |

Input examples:

```bash
printf 'abc' | agent-exec run --stdin - -- cat

agent-exec run --stdin - -- cat <<'EOF'
line1
line2
EOF

agent-exec run --stdin "abc" -- cat
agent-exec run --stdin-file ./input.txt -- cat
```

If `--stdin -` receives a terminal instead of redirected input, the command fails with `error.code` set to `stdin_required`.

### `status`: read job state

```bash
agent-exec status <JOB_ID>
```

The response can report `created`, `running`, `exited`, `killed`, or `failed`. It always includes `job_id`, `state`, and `created_at`; it includes `started_at`, `finished_at`, and `exit_code` when available.

### `tail`: read bounded output tails

```bash
agent-exec tail [--tail-lines <N>] [--max-bytes <N>] [--compress <MODE>] <JOB_ID>
```

The response includes bounded `stdout` and `stderr` tails, their raw byte ranges and totals, `encoding`, and both log paths. Defaults are 50 lines and 65,536 bytes per stream.

### `wait`: observe until completion or deadline

```bash
agent-exec wait [--until <SECONDS> | --forever] [--poll <SECONDS>] <JOB_ID>
```

The default client-side deadline is 30 seconds. Reaching it does not stop the job. Use `run --timeout` to limit process runtime.

### `kill`: send a signal

```bash
agent-exec kill [--signal <NAME>] [--no-wait] <JOB_ID>
```

The default signal is `TERM`. By default, `kill` briefly observes the result; `--no-wait` skips that observation.

### `list`: list jobs

```bash
agent-exec list [--state <STATE>] [--limit <N>] [--cwd <PATH> | --all] [--tag <PATTERN>]...
```

By default, `list` returns jobs whose persisted current working directory matches the caller's current working directory. `--cwd` selects another directory, and `--all` disables current working directory filtering. States are `created`, `running`, `exited`, `killed`, `failed`, and `unknown`.

Repeated `--tag` filters use logical AND. An exact pattern such as `ci` matches that tag only. A namespace pattern such as `project.build.*` matches tags below that namespace.

```bash
agent-exec list --all --tag ci
agent-exec list --all --tag project.build.*
agent-exec list --tag ci --tag release
```

### `ps`: list running jobs

```bash
agent-exec ps [--limit <N>] [--cwd <PATH> | --all] [--tag <PATTERN>]...
```

`ps` is equivalent to `list --state running` and returns the same `type` set to `list`.

### `tag set`: replace job tags

```bash
agent-exec tag set <JOB_ID> [--tag <TAG>]...
```

The command replaces all tags, preserving the first occurrence of each duplicate. Omit `--tag` to clear the list.

```bash
agent-exec run --tag project.build --tag ci -- make build
agent-exec tag set 7f3a9c1e4b2d8a605e7c9f0134ab6d82 --tag project.release --tag approved
agent-exec tag set 7f3a9c1e4b2d8a605e7c9f0134ab6d82
```

Stored tags use dot-separated segments containing alphanumeric characters and hyphens, such as `ci`, `project.build`, and `release.v2`. The `.*` suffix is reserved for filter patterns.

### `notify set`: update notification metadata

```bash
agent-exec notify set <JOB_ID> [OPTIONS]
```

This metadata-only command updates the persisted notification configuration. It never invokes a sink immediately, including for a terminal job. Unspecified fields are preserved. `--command` replaces `notify_command`, while `notify_file` remains unchanged.

Completion notification:

```bash
JOB=$(agent-exec run --no-wait -- sleep 5 | jq -r .job_id)
agent-exec notify set "$JOB" --command 'cat > /tmp/event.json'
```

Output-match notification:

```bash
JOB=$(agent-exec run --no-wait -- sh -c 'sleep 1; echo ERROR foo' | jq -r .job_id)
agent-exec notify set "$JOB" \
  --output-pattern 'ERROR' \
  --output-command 'cat >> /tmp/matches.ndjson'

agent-exec notify set "$JOB" \
  --output-pattern '^ERR' \
  --output-match-type regex \
  --output-stream stderr \
  --output-file /tmp/stderr-matches.ndjson
```

Output-match settings apply only to lines observed after the configuration becomes active. Use `agent-exec notify set --help` for all fields.

### `gc`: collect old job data

```bash
agent-exec gc [--older-than <DURATION>] [--max-jobs <N>] [--max-bytes <BYTES>] [--dry-run]
```

`gc` scans the entire jobs root, regardless of the current working directory. In the current implementation, it first builds an oldest-first pool of terminal jobs older than `--older-than`; `--max-jobs` and `--max-bytes` apply only within that pool and never select newer terminal jobs. Jobs in `created` or `running` state and jobs with unreadable state are preserved.

Selection is order-dependent: age eligibility is established first, `--max-jobs` removes age-only selection from the newest `N` pool entries, and `--max-bytes` may then select oldest pool entries until their removal would bring pool storage within the limit.

| Flag | Default | Description |
|------|---------|-------------|
| `--older-than <DURATION>` | `30d` | Build the age-eligible terminal pool. Accepted suffixes include `d`, `h`, `m`, and `s`. |
| `--max-jobs <N>` | None | Apply count policy within the age-eligible pool. |
| `--max-bytes <BYTES>` | None | Apply oldest-first byte policy within the age-eligible pool. |
| `--dry-run` | `false` | Report aggregate effects without deleting directories. |

The age timestamp is `finished_at` when present, otherwise `updated_at`.

```bash
agent-exec gc --dry-run
agent-exec gc --older-than 7d --dry-run
agent-exec gc --older-than 7d
agent-exec --root /tmp/jobs gc --older-than 7d
```

The `gc` response is aggregate-only and has no `jobs` array:

| Field | Description |
|-------|-------------|
| `root` | Resolved jobs root. |
| `dry_run` | Whether deletion was disabled. |
| `older_than` | Effective retention duration. |
| `older_than_source` | `default` or `flag`. |
| `deleted` | Directories actually deleted; always `0` for a dry run. |
| `skipped` | Total skipped directories; equal to `out_of_scope + failed`. |
| `out_of_scope` | Entries excluded by state, age, timestamp, readability, or policy limits. |
| `failed` | Eligible entries that could not be deleted or remained after deletion. |
| `freed_bytes` | Bytes removed, or bytes that a dry run would remove. |
| `scanned_dirs` | Directories scanned. |
| `candidate_count` | Directories selected by policy before deletion limits. |

### `delete`: remove explicit or current-directory jobs

```bash
agent-exec delete <JOB_ID> [--dry-run]
agent-exec delete --all [--dry-run]
```

`rm` is a visible alias and returns the same response shape.

Single-job deletion is not scoped by the current working directory. The implementation rejects a job whose readable state is `running`. It may delete `created`, terminal, or `unknown` jobs, including a directory whose state is missing or unreadable. Inspect the job or use `--dry-run` before deleting an explicit ID when state integrity is uncertain.

`delete --all` removes only terminal jobs whose persisted current working directory matches the caller's current working directory. It skips `created`, `running`, unreadable-state, and terminal jobs whose recorded PID is still alive. Jobs from other directories contribute to `out_of_scope` but are not listed individually.

```bash
agent-exec delete 7f3a9c1e4b2d8a605e7c9f0134ab6d82
agent-exec delete --all --dry-run
agent-exec delete --all
agent-exec --root /tmp/jobs delete --all
```

The response includes `root`, `dry_run`, `deleted`, `skipped`, `out_of_scope`, `failed`, and per-job `jobs`. `cwd_scope` is present only for `--all`. Each job result contains `job_id`, `state`, `action`, and `reason`; `action` is `deleted`, `would_delete`, or `skipped`. A reported `deleted` action means the path was confirmed absent after deletion.

### `schema`: print the response schema

```bash
agent-exec schema
```

The response contains the JSON Schema document for CLI response types, its schema format, and generation timestamp.

## Automatic Cleanup

After a successful launch, `run`, `start`, and `restart` perform bounded, best-effort automatic GC by default.

- Default retention is `30d`.
- `created`, `running`, and unreadable-state jobs are preserved.
- Cleanup failure does not fail the launch command.
- Scan and deletion limits bound launch-time work.

Per-invocation controls are `--no-auto-gc`, `--auto-gc-older-than`, `--auto-gc-max-jobs`, and `--auto-gc-max-bytes`.

```toml
[gc]
auto = true
older_than = "30d"
max_jobs = 200
max_bytes = 1073741824
scan_limit = 200
delete_limit = 20
```

CLI values override configuration for that invocation.

## HTTP Server

`agent-exec serve` exposes job operations to HTTP clients.

```bash
agent-exec serve [--bind <HOST:PORT> | --port <PORT>] [--allow-origin <ORIGIN>]
```

The default address is `127.0.0.1:19263`. `--port` changes the loopback port. A non-loopback bind requires both `--insecure` and a nonempty `AGENT_EXEC_SERVE_TOKEN`.

### Security model

`AGENT_EXEC_SERVE_TOKEN` enables bearer authentication only for the mutating endpoints `POST /exec` and `POST /kill/{id}`. It does not protect `GET /health`, `GET /status/{id}`, `GET /tail/{id}`, or `GET /wait/{id}`. Those read endpoints can expose job state and output.

Keep the default loopback bind unless remote access is required. For non-loopback access, restrict the port with a firewall, private network, or authenticating reverse proxy. Do not expose the server directly to the public internet. The required `--insecure` flag acknowledges that the built-in token does not secure read endpoints.

`--allow-origin` enables CORS for one explicit origin. The wildcard origin `*` is rejected.

### Endpoints

| Method | Path | CLI equivalent | Behavior |
|--------|------|----------------|----------|
| `GET` | `/health` | None | Returns `schema_version`, `ok`, and `type` set to `health`. |
| `POST` | `/exec` | `run` | Starts a job and returns `RunData`. |
| `GET` | `/status/{id}` | `status` | Returns job status. |
| `GET` | `/tail/{id}` | `tail` | Returns bounded `stdout` and `stderr` tails. |
| `GET` | `/wait/{id}` | `wait --forever` | Blocks until a terminal state. |
| `POST` | `/kill/{id}` | `kill` | Sends `TERM`; `?no_wait=true` skips observation. |

HTTP responses use the same `schema_version`, `ok`, and `type` envelope fields as CLI responses.

### `POST /exec` request

```json
{
  "command": ["bash", "-c", "echo hello"],
  "cwd": "/tmp",
  "env": {"FOO": "bar"},
  "timeout": 30,
  "wait": true,
  "until": 10,
  "max_bytes": 65536
}
```

Only `command` is required. Pass `timeout` as a nonnegative number of seconds; it may be fractional. `until` must be a nonnegative integer number of seconds. `wait` defaults to `true`, `until` to `10`, and `max_bytes` to `65536`. The obsolete `timeout_ms` field is rejected.

### Docker client example

Start the host server on a non-loopback address with a strong token:

```bash
export AGENT_EXEC_SERVE_TOKEN="$(openssl rand -hex 32)"
agent-exec serve --bind 0.0.0.0:19263 --insecure
```

Provide the same token to the container through its secret or environment configuration. From a Docker container on a platform that supports `host.docker.internal`:

```bash
curl -sS \
  -H "Authorization: Bearer $AGENT_EXEC_SERVE_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"command":["my-agent-script"],"wait":false,"until":0,"max_bytes":65536}' \
  http://host.docker.internal:19263/exec
```

Use the returned job ID with the read endpoints. Because read endpoints do not require the bearer token, network restrictions remain mandatory.

## MCP Server

`agent-exec mcp` exposes the managed-job lifecycle over stdio. It uses the same jobs root, persisted metadata, detached supervisor, logs, and response envelopes as the CLI; it does not require the HTTP server.

For tested setup instructions for Claude Code, Codex CLI, OpenCode, and Hermes Agent, see [AI agent integrations](docs/agent-integrations.md).

Configure an MCP client to launch:

```text
command: agent-exec
args: ["mcp"]
```

Set observation limits in the MCP server process environment:

```text
AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS=10
AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55
```

`AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS` supplies an omitted `until` for MCP `run` and `wait`. `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` caps every MCP observation duration. Set the maximum below the MCP client's request timeout. An over-limit request uses the cap; it does not fail or stop the job.

Use a nondefault jobs root only when needed:

```text
command: agent-exec
args: ["--root", "/path/to/jobs", "mcp"]
```

### Hermes Native MCP configuration

```yaml
mcp_servers:
  agent-exec:
    command: agent-exec
    args: ["mcp"]
    env:
      AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS: "10"
      AGENT_EXEC_MCP_MAX_UNTIL_SECONDS: "55"
```

When MCP is unavailable, use `agent-exec run -- <command>` with CLI observation commands.

### MCP tools

| Tool | Parameters | Behavior |
|------|------------|----------|
| `run` | `command: string[]`, `cwd?: string`, `env?: object`, `timeout?: integer`, `until?: integer` | Starts a detached job. `timeout` and `until` are seconds; the legacy omitted `until` is 10 seconds unless configured. |
| `status` | `job_id: string` | Returns canonical job status. |
| `tail` | `job_id: string`, `lines?: integer`, `max_bytes?: integer` | Reads bounded tails; defaults are 50 lines and 65,536 bytes. |
| `wait` | `job_id: string`, `until?: integer` | Observes for a bounded duration; the legacy omitted `until` is 30 seconds unless configured. Indefinite MCP waits are not supported. |
| `kill` | `job_id: string` | Sends `TERM`. |

Retain the job ID returned by `run`. Closing the MCP transport, reaching an observation deadline, receiving no output, or encountering a tool error does not stop the job. Use `kill` only for explicit cancellation.

The MCP `run` tool intentionally omits CLI-only input, masking, notification, tag, compression, and shell-wrapper controls.

For an MCP host with a 60-second request deadline, a maximum of 55 seconds leaves time for the response to return. The default can remain shorter, such as 10 seconds.

## Configuration

`agent-exec` reads optional `[shell]`, `[gc]`, and `[compression]` sections from `$XDG_CONFIG_HOME/agent-exec/config.toml`, which normally resolves to `~/.config/agent-exec/config.toml`.

```toml
[shell]
unix = ["sh", "-lc"]
windows = ["cmd", "/C"]

[gc]
auto = true
older_than = "30d"
max_jobs = 200
max_bytes = 1073741824
scan_limit = 200
delete_limit = 20

[compression]
default = "route"
```

All keys are optional. Shell values fall back to `sh -lc` on Unix-like systems and `cmd /C` on Windows. GC and compression values fall back to their built-in defaults.

### Shell wrapper precedence

1. `--shell-wrapper <PROGRAM AND FLAGS>`
2. The file selected by `--config <PATH>`, when specified
3. The default XDG configuration file, only when `--config` is omitted
4. The built-in platform wrapper

### Command launch modes on Unix

The number of command arguments after `--` selects the launch mode:

| Mode | Example | Behavior |
|------|---------|----------|
| Shell string | `agent-exec run -- "echo hi && ls"` | A single argument is passed unchanged to the shell wrapper. |
| `argv` | `agent-exec run -- cargo test` | Two or more arguments use an `exec "$@"` handoff after the wrapper initializes the shell environment. |

The `argv` handoff replaces the wrapper with the target workload, so PID and lifecycle tracking align with the command. Prefer `argv` for routine commands and shell-string mode for actual shell expressions.

The configured wrapper also runs `--notify-command`. Notification delivery always uses shell-string mode.

```bash
agent-exec run --shell-wrapper "bash -lc" -- ./my_script.sh
agent-exec run --config /path/to/config.toml -- ./my_script.sh
```

## Job Completion Events

`--notify-command` and `--notify-file` deliver a `job.finished` event after a launched job reaches a terminal state.

- `--notify-command` runs a shell command through the configured wrapper and writes event JSON to its standard input.
- `--notify-file` appends one NDJSON line.
- `completion_event.json` stores the event and sink delivery results in the job directory.
- Delivery is best effort; sink failure does not change job state.
- Inspect `completion_event.json.delivery_results` when delivery success matters.

```bash
JOB=$(agent-exec run --notify-file /tmp/agent-exec-events.ndjson -- echo hello | jq -r .job_id)
agent-exec wait "$JOB"
agent-exec tail "$JOB"

JOB=$(agent-exec run --notify-command 'cat > /tmp/agent-exec-event.json' -- echo hello | jq -r .job_id)
agent-exec wait "$JOB"
```

Command sinks receive these environment variables:

- `AGENT_EXEC_EVENT_PATH`: persisted `completion_event.json` or `notification_events.ndjson` path
- `AGENT_EXEC_JOB_ID`: canonical job ID
- `AGENT_EXEC_EVENT_TYPE`: `job.finished` or `job.output.matched`

Example payload:

```json
{
  "schema_version": "0.1",
  "event_type": "job.finished",
  "job_id": "7f3a9c1e4b2d8a605e7c9f0134ab6d82",
  "state": "exited",
  "command": ["echo", "hello"],
  "cwd": "/path/to/current-working-directory",
  "started_at": "2026-07-19T12:00:00Z",
  "finished_at": "2026-07-19T12:00:00Z",
  "duration_ms": 12,
  "exit_code": 0,
  "stdout_log_path": "/jobs/7f3a9c1e4b2d8a605e7c9f0134ab6d82/stdout.log",
  "stderr_log_path": "/jobs/7f3a9c1e4b2d8a605e7c9f0134ab6d82/stderr.log"
}
```

For signal termination, `state` is `killed`, `exit_code` may be absent, and `signal` is present when available.

## Install Embedded Skill

`install-skills` installs only the embedded `agent-exec` skill into `.agents/skills/` or `.claude/skills/` and updates the corresponding `.skill-lock.json`. It is not a general skill installer and does not accept external sources.

```bash
agent-exec install-skills
agent-exec install-skills --claude
agent-exec install-skills --claude --global
```

## OpenClaw Integration

### Return completion to the launching session

A completion callback can return the job ID and event path to the OpenClaw session that launched the work. The session can inspect the persisted event and logs before responding.

```bash
SESSION_ID="01bb09d5-6485-4a50-8d3b-3f6e80c61f9c"
REPLY_CHANNEL="telegram"

agent-exec run \
  --notify-command "openclaw agent --deliver --reply-channel $REPLY_CHANNEL --session-id $SESSION_ID -m \"job_id=\$AGENT_EXEC_JOB_ID event_path=\$AGENT_EXEC_EVENT_PATH\"" \
  -- ./scripts/run-heavy-task.sh
```

When both agents share a filesystem, sending `job_id` and `event_path` is more compact than embedding the full event JSON.

### Add a callback to a running job

```bash
JOB=$(agent-exec run --no-wait -- ./scripts/run-heavy-task.sh | jq -r .job_id)
SESSION_ID="01bb09d5-6485-4a50-8d3b-3f6e80c61f9c"
REPLY_CHANNEL="telegram"

agent-exec notify set "$JOB" \
  --command "openclaw agent --deliver --reply-channel $REPLY_CHANNEL --session-id $SESSION_ID -m \"job_id=\$AGENT_EXEC_JOB_ID event_path=\$AGENT_EXEC_EVENT_PATH\""
```

`notify set` updates future delivery metadata and does not invoke the callback immediately.

### Use a durable file handoff

```bash
agent-exec run \
  --notify-file /var/lib/agent-exec/events.ndjson \
  -- ./scripts/run-heavy-task.sh
```

A separate worker can process the NDJSON file, retry delivery, and route events without coupling that work to the supervisor.

Keep command sinks short, fast, and idempotent. Common failures include quoting errors, environment or `PATH` differences, nonzero downstream exits, and incorrect delivery targets. Use a checked-in helper or durable worker for substantial orchestration.

## Output-Match Events

When output-match notification metadata is active, the supervisor evaluates newly observed lines from `stdout`, `stderr`, or either stream and emits `job.output.matched` for every match.

- `contains` performs substring matching.
- `regex` uses Rust regular-expression syntax.
- Earlier output is not replayed after `notify set`.
- Sink failure is recorded in `notification_events.ndjson` and does not affect job state.
- `completion_event.json` remains reserved for `job.finished` delivery results.

```json
{
  "schema_version": "0.1",
  "event_type": "job.output.matched",
  "job_id": "7f3a9c1e4b2d8a605e7c9f0134ab6d82",
  "pattern": "ERROR",
  "match_type": "contains",
  "stream": "stdout",
  "line": "ERROR: connection refused",
  "stdout_log_path": "/jobs/7f3a9c1e4b2d8a605e7c9f0134ab6d82/stdout.log",
  "stderr_log_path": "/jobs/7f3a9c1e4b2d8a605e7c9f0134ab6d82/stderr.log"
}
```

## Logging

Diagnostic logs use `stderr` only:

```bash
RUST_LOG=debug agent-exec run -- echo hello
agent-exec -v run -- echo hello
```

## Development

```bash
cargo build
cargo test --all
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```
