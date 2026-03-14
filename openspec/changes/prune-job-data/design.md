# Design: prune-job-data

## Overview

This change adds an explicit `agent-exec gc` command that prunes old terminal job directories from the resolved jobs root. The design keeps garbage collection out of `run`, `list`, and background supervision so deletion remains operator-triggered, predictable, and easy to audit.

## Why A Single Proposal

CLI surface, retention policy, and jobstore deletion semantics are tightly coupled. Shipping only part of the change would either expose an unusable command or leave deletion behavior undocumented, so the proposal remains a single unit.

## Command Shape

Proposed MVP:

```text
agent-exec gc [--root <path>] [--older-than <duration>] [--dry-run]
```

- `--older-than <duration>` is optional; when omitted, GC uses a default retention window of `30d`.
- `--dry-run` switches the command into preview mode while preserving the same traversal and reporting logic.
- No cwd filter is included in MVP; GC operates on the entire resolved root to avoid coupling retention to the caller's current directory.

## Eligibility Rules

Only directories that can be read as valid jobs are considered. Eligibility is determined as follows:

1. Read `meta.json` and `state.json`.
2. If either file is unreadable or unparsable, skip the directory.
3. If `job.status == running`, skip the job.
4. If `job.status` is one of `exited|killed|failed`, determine the GC timestamp:
   - use `finished_at` when present
   - otherwise use `updated_at`
   - otherwise skip the job
5. Delete only when the GC timestamp is older than the provided cutoff.

This preserves safety for incomplete or legacy records while still allowing cleanup of terminal jobs whose finish timestamp was not persisted.

## Reporting Model

The command should produce a machine-friendly JSON response with:

- resolved `root`
- `dry_run`
- `older_than`
- `older_than_source` (for example `default` or `flag`) so callers can tell whether `30d` came from the built-in default or explicit input
- summary counters such as `deleted`, `skipped`, and `freed_bytes`
- per-job results including `job_id`, `state`, `action`, `reason`, and `bytes`

Per-job reporting matters because destructive mode is the default. Callers need to distinguish "was deleted", "would be deleted", and "was skipped for safety" without scraping stderr.

## Deletion Semantics

- Deletion removes the entire `<root>/<job_id>/` directory recursively, including `completion_event.json` when present.
- Byte accounting should be computed before deletion so the response can report reclaimed space.
- Once deleted, existing commands continue to behave naturally because `JobDir::open` already maps missing directories to `job_not_found`.

## Testing Strategy

Integration tests should create isolated roots and cover:

- old terminal jobs are deleted in default mode
- `--dry-run` preserves directories but reports the same candidates
- running jobs are never deleted
- jobs missing `finished_at` but having old `updated_at` are deleted
- jobs missing both timestamps or containing unreadable state are skipped safely
