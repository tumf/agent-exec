# CLI Contract

Use this reference for CLI fallback, configuration, command defaults, and response details. MCP clients should discover tool arguments and result schemas from the MCP server instead of duplicating them here.

## CLI fallback

When MCP is unavailable, start with:

```bash
agent-exec run -- <command>
```

Pass ordinary arguments directly after `--`. Use `sh -lc` only when pipes, redirects, variable expansion, or compound shell syntax are required. Do not add timing, notification, masking, or shell-wrapper options without a concrete need.

Examples:

```bash
agent-exec run -- make test
agent-exec run -- npm run build
agent-exec run -- cargo test
```

## MCP server configuration

Configure agent-exec as a stdio MCP server:

```yaml
mcp_servers:
  agent-exec:
    command: agent-exec
    args: ["mcp"]
```

Use `args: ["--root", "/path/to/jobs", "mcp"]` only when a non-default jobs root is required.

Each MCP host must set its own safe observation ceiling through `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS`. This value becomes the default and maximum `until` for MCP `run` and `wait`; agent-exec does not infer the host timeout or reserve a safety margin. For example, a host with a fixed 60-second request deadline can set `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`.

## Successful responses

Expect one JSON object on stdout for every successful command:

```json
{
  "schema_version": "0.3",
  "ok": true,
  "type": "<command>",
  "...": "command-specific fields"
}
```

Key success payloads:

- `run`: returns inline output (`stdout`, `stderr`, ranges, total bytes) with default wait budget
- `status`: returns `job_id`, `state`, `created_at`, and optional terminal fields, plus schema `0.3` execution diagnostics (`command`, `cwd`, `tags`, `pid`, `process_alive`, `updated_at`, `elapsed_ms`, `duration_ms`, `signal`, `logs_drained`, log paths, and log byte totals)
- `tail`: returns `stdout`, `stderr`, range fields, truncation flag, and total byte counts
- `wait`: returns terminal `state` and optional `exit_code`
- `kill`: returns `job_id` and requested `signal`
- `list`: returns `root`, `jobs`, `truncated`, and `skipped`
- `install-skills`: returns installed skill summaries plus `lock_file_path`

## Error responses

Expect this envelope for expected failures:

```json
{
  "schema_version": "0.3",
  "ok": false,
  "type": "error",
  "error": {
    "code": "<error_code>",
    "message": "<description>",
    "retryable": false
  }
}
```

Common exit codes:

- `0`: success
- `1`: expected failure with JSON error on stdout
- `2`: clap usage error

## `run` notes

- In normal harness use, start with plain `agent-exec run -- <command>`; the default behavior is the optimized path.
- Pass the workload as normal argv after `--`. Do not prepend `sh -lc` for ordinary commands; reserve that for cases that truly need shell parsing.
- `run` waits up to 10 seconds by default and returns inline output; use `--no-wait` only when immediate return is more important than seeing startup output.
- The inline stdout/stderr payload is only a partial view. Use returned log paths and follow-up commands when you need the full output.
- Use `wait` when terminal state is required and `tail` for tail-side observation.
- Use `--mask KEY` when secrets are present in `--env`; masked values become `***` in output and persisted metadata.
- `--abandon-job-after <SECONDS>` gives up on the job, terminates it, and may permanently lose unfinished results. It is not an observation deadline; `--until` bounds observation without stopping the job. It requires `--acknowledge-result-loss`, and the default is no limit. The removed `--timeout` spelling always fails with migration guidance and never creates a job.

## `abandon` notes

- `agent-exec abandon set <JOB_ID> --in <SECONDS> --acknowledge-result-loss` replaces a running job's abandonment deadline; `--in` is measured from durable acceptance of the update, not from job start. It is as destructive as `--abandon-job-after`, so it always requires `--acknowledge-result-loss`.
- `agent-exec abandon clear <JOB_ID>` removes the deadline. It never signals the job, so it needs no acknowledgement.
- Both are refused with `invalid_state` for created, terminal, and already-triggered jobs, and for a running job launched by an older release that has no supervisor-authored control record. In that last case the error tells you to `agent-exec restart` the job; do not treat a refusal as a partially applied change, because a rejected operation mutates nothing.
- Once abandonment has begun, a later change fails rather than claiming it prevented the abandonment. Do not report to the user that a deadline was extended unless the response carried an `abandon_revision`.

## `status` notes

- `state` is the persisted lifecycle state; `status` never rewrites it.
- `process_alive` is a separate best-effort probe resolved only for persisted `running` state. It is omitted otherwise, and omission means no live observation was made.
- A stale running job reads as `state="running"` with `process_alive=false` here, and as `unknown` in `list`.
- `elapsed_ms` is live (response time minus `started_at`) and appears only for non-terminal started jobs; `duration_ms` is the persisted terminal duration.
- Log byte totals come from file metadata only. `status` never reads log contents; use `tail` for output.
- `status` never exposes environment values, stdin content, notification secrets, or shell-expanded commands.
- `abandoned_by="abandon_job_after"` with `result_loss=true` appears only when a configured `--abandon-job-after` limit actually terminated the job. The terminal `state` value is unchanged, and both markers are absent when the limit never fired. `list` reports the same pair.
- `abandon_job_after_ms`, `abandon_deadline`, `abandon_remaining_ms`, `abandon_revision`, and `abandon_configured_by` report the deadline the supervisor will actually act on, including one changed at runtime. All five are null when the job has no supervisor-authored control record; duration, deadline, and remaining time are also null when the control is disabled, and remaining time is null for terminal jobs. `abandon_remaining_ms` is advisory: it is computed at response time and clamped to `[0, abandon_job_after_ms]`, and the supervisor decides firing from its own locked record.

## `list` notes

- `list` filters by the caller's current working directory by default.
- Use `--cwd <dir>` for an explicit directory filter.
- Use `--all` to disable cwd filtering.

## `install-skills` notes

- Expect `skills[*].name`, `skills[*].source_type`, and `skills[*].path` in the success payload.
- Expect `lock_file_path` to point at the updated `.agents/.skill-lock.json` file.
- Use `--global` when the skill should be installed into `~/.agents/` instead of the current directory.
- `install-skills` installs only the built-in `agent-exec` skill embedded in the binary.
