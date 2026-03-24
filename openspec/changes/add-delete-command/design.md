# Design: add-delete-command

## Context

`gc` already deletes terminal job directories, but it is retention-based and traverses the whole root. The requested workflow is different: remove one known job immediately, or clear finished jobs that belong to the current working directory. That makes `delete` an operator-driven cleanup command rather than a retention policy.

## Goals

- Provide a fast explicit path for removing one job by id.
- Provide a cwd-scoped cleanup path for finished jobs without touching unrelated directories.
- Preserve safety by refusing to delete running jobs.
- Keep the response shape easy for agents to parse.

## Non-Goals

- Replacing `gc` or changing its age-based semantics.
- Deleting running jobs.
- Introducing background or automatic cleanup.

## Command Shape

```text
agent-exec delete <JOB_ID>
agent-exec delete --all [--dry-run]
```

Single-job mode and bulk mode should be mutually exclusive. Bulk mode targets jobs whose persisted `meta.json.cwd` matches the caller's current working directory, using the same normalization strategy already established for `list`.

## State Rules

- `delete <JOB_ID>`
  - allowed: `created`, `exited`, `killed`, `failed`
  - rejected: `running`
- `delete --all`
  - deleted: `exited`, `killed`, `failed` in the scoped cwd
  - skipped: `created`, `running`, unreadable/incomplete records

This split keeps the bulk command aligned with the user's "delete finished jobs in this cwd" request while still allowing explicit removal of an unstarted `created` job.

## Response Shape

The command should return a single structured envelope similar to `gc`:

- `root`
- `dry_run`
- `deleted`
- `skipped`
- `jobs[]` with `job_id`, `state`, `action`, `reason`

Using one response shape for both single-job and bulk modes keeps downstream parsing simple.

## Verification Strategy

- Integration tests should prove that bulk deletion is limited to the current cwd.
- Integration tests should prove that running jobs are never removed.
- Integration tests should prove that dry-run preserves directories.
- Integration tests should prove that deleted jobs become `job_not_found` to existing lookup commands.
