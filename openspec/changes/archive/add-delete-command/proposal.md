# Change Proposal: add-delete-command

## Problem/Context
- `agent-exec gc` provides age-based cleanup across the whole jobs root, but it does not cover the interactive cleanup workflow where a user wants to remove a specific job immediately or clear finished jobs from the current working directory.
- `list` already treats the current working directory as the default scope, so users reasonably expect cleanup commands to support the same cwd-oriented workflow.
- Deleting job directories is destructive, so the CLI should keep the behavior explicit and predictable instead of overloading `kill` or changing `gc` semantics.

## Proposed Solution
- Add a new `agent-exec delete` subcommand for explicit job-directory removal.
- Support `agent-exec delete <job_id>` for single-job deletion. The command deletes `created`, `exited`, `killed`, and `failed` jobs, and rejects `running` jobs with a stable API error.
- Support `agent-exec delete --all` for bulk deletion of finished jobs in the caller's current working directory. In bulk mode, only terminal jobs (`exited`, `killed`, `failed`) are deleted; `running` and `created` jobs are skipped.
- Add `--dry-run` so callers can preview which jobs would be deleted before making changes.
- Return a structured machine-readable response that reports deleted and skipped jobs in the same spirit as `gc`.

## Acceptance Criteria
- `agent-exec delete <job_id>` removes the specified non-running job directory and subsequent `status`/`tail`/`wait`/`kill` calls treat it as `job_not_found`.
- `agent-exec delete <job_id>` rejects a running job without deleting its directory.
- `agent-exec delete --all` deletes only terminal jobs whose persisted `meta.json.cwd` matches the caller's current working directory.
- `agent-exec delete --all` does not delete `created` or `running` jobs, and reports those decisions clearly in the response when encountered in the scoped set.
- `agent-exec delete --dry-run ...` performs no deletion and reports the same candidate set it would act on.
- README and OpenSpec document the new command and its cwd-scoped bulk-delete semantics.

## Out of Scope
- Automatic deletion triggered by `run`, `wait`, `kill`, or background supervision.
- Force-deleting running jobs or combining `kill` and deletion into one command.
- Root-wide bulk deletion or age-based retention changes to `gc`.
