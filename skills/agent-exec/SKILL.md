# agent-exec Skill

Agent skill for running background jobs and managing their lifecycle via the `agent-exec` CLI.

## Overview

`agent-exec` is a non-interactive job runner designed for use by AI agents. All output is JSON-only on stdout; diagnostic logs go to stderr.

## Commands

### `run` — Start a background job

```bash
agent-exec run [OPTIONS] -- <COMMAND> [ARGS...]
```

Returns a JSON response immediately with a `job_id`. Use `--wait` to block until completion.

Key options:
- `--root <DIR>` — Override the jobs root directory
- `--snapshot-after <MS>` — Wait N ms before returning with a log snapshot (default: 10000)
- `--tail-lines <N>` — Lines to include in snapshot (default: 50)
- `--timeout <MS>` — Kill the job after N ms (0 = no timeout)
- `--cwd <DIR>` — Working directory for the command
- `--env KEY=VALUE` — Set an environment variable (repeatable)
- `--mask KEY` — Mask a secret value in output (repeatable)
- `--wait` — Block until the job finishes

### `status` — Get job status

```bash
agent-exec status <JOB_ID>
```

### `tail` — Read stdout/stderr tail

```bash
agent-exec tail [--tail-lines N] <JOB_ID>
```

### `wait` — Wait for a job to finish

```bash
agent-exec wait [--timeout-ms N] <JOB_ID>
```

### `kill` — Send a signal to a job

```bash
agent-exec kill [--signal TERM|INT|KILL] <JOB_ID>
```

### `list` — List all jobs

```bash
agent-exec list [--state running|exited|killed|failed] [--cwd DIR]
```

### `install-skills` — Install agent skills

```bash
agent-exec install-skills [--source self|local:<path>] [--global]
```

Installs agent skills into `.agents/skills/` (or `~/.agents/skills/` with `--global`).
Updates `.agents/.skill-lock.json` to track installed skills.

Sources:
- `self` — Install the built-in `agent-exec` skill (default)
- `local:<path>` — Install a skill from a local directory

## JSON Response Format

All responses share a common envelope:

```json
{
  "schema_version": "0.1",
  "ok": true,
  "type": "<command>",
  ...
}
```

Errors:

```json
{
  "schema_version": "0.1",
  "ok": false,
  "type": "error",
  "error": {
    "code": "<error_code>",
    "message": "<description>",
    "retryable": false
  }
}
```

## Environment Variables

- `AGENT_EXEC_ROOT` — Override the default jobs root directory

## Exit Codes

- `0` — Success
- `1` — Expected failure (JSON error response emitted to stdout)
- `2` — Usage error (clap argument parsing failure)
