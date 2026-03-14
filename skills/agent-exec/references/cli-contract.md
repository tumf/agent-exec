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

- `run`: returns `job_id`, `state`, log paths, timing fields, optional `snapshot`, and optional terminal fields when `--wait` is used
- `status`: returns `job_id`, `state`, `started_at`, and optional terminal fields
- `tail`: returns `stdout_tail`, `stderr_tail`, truncation details, and observed byte counts
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

- Use `--wait` when the initial response must include terminal fields like `exit_code`, `finished_at`, and `final_snapshot`.
- Use `--mask KEY` when secrets are present in `--env`; masked values become `***` in output and persisted metadata.
- Use `--snapshot-after 0` when immediate return matters more than an initial tail snapshot.

## `list` notes

- `list` filters by the caller's current working directory by default.
- Use `--cwd <dir>` for an explicit directory filter.
- Use `--all` to disable cwd filtering.
