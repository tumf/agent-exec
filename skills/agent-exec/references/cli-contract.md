# CLI Contract

## Successful responses

Expect one JSON object on stdout for every successful command:

```json
{
  "schema_version": "0.1",
  "ok": true,
  "type": "<command>",
  "...": "command-specific fields"
}
```

Key success payloads:

- `run`: returns inline output (`stdout`, `stderr`, ranges, total bytes) with default wait budget
- `status`: returns `job_id`, `state`, `started_at`, and optional terminal fields
- `tail`: returns `stdout`, `stderr`, range fields, truncation flag, and total byte counts
- `wait`: returns terminal `state` and optional `exit_code`
- `kill`: returns `job_id` and requested `signal`
- `list`: returns `root`, `jobs`, `truncated`, and `skipped`
- `install-skills`: returns installed skill summaries plus `lock_file_path`

## Error responses

Expect this envelope for expected failures:

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

## `list` notes

- `list` filters by the caller's current working directory by default.
- Use `--cwd <dir>` for an explicit directory filter.
- Use `--all` to disable cwd filtering.

## `install-skills` notes

- Expect `skills[*].name`, `skills[*].source_type`, and `skills[*].path` in the success payload.
- Expect `lock_file_path` to point at the updated `.agents/.skill-lock.json` file.
- Use `--global` when the skill should be installed into `~/.agents/` instead of the current directory.
- `install-skills` installs only the built-in `agent-exec` skill embedded in the binary.
