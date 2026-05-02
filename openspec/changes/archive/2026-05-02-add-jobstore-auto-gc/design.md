# Design: Bounded jobstore auto-GC

## Goals

- Keep the default jobs root from growing indefinitely under routine agent use.
- Preserve `run` / `start` as one-round-trip launch-and-observe commands.
- Reuse manual `gc` safety and observability rules instead of creating a second cleanup policy.
- Keep existing job directory layout and lookup compatibility intact.

## Non-Goals

- No date/cwd directory sharding in this change.
- No background service, daemon, cron, or launchd integration.
- No cleanup of active jobs.
- No change to stdout being one JSON object per command.

## Architecture

### Shared GC engine

Refactor `src/gc.rs` so command handling is thin and cleanup behavior lives in reusable planner/executor components.

Recommended internal concepts:

- `GcPolicy`: retention cutoff, max terminal jobs, max root bytes, dry-run, automatic/manual mode, scan/deletion budget.
- `GcCandidate`: job id, path, state, timestamps, bytes, eligibility reasons.
- `GcSummary`: scanned dirs, job dirs, non-job dirs, state counts, bytes, candidate counts, deletion results.
- `GcOutcome`: existing per-job results plus the summary needed for JSON output and optional run/start metadata.

Manual `gc` should use an unbounded or explicitly requested policy. Auto-GC should use the same engine with strict defaults.

### Auto-GC call sites

Auto-GC should run only after the current command's job directory has been created and the workload launch path has succeeded enough that the current job is safely represented on disk.

- `run`: after supervisor launch and before or after inline observation, depending on which ordering keeps the response latency lower and avoids deleting the current job. The current running job must be skipped by state.
- `start`: after the created job transitions to running and before response emission, using the same best-effort behavior.

If auto-GC fails, the parent command should log diagnostics to stderr via tracing but still return the normal `run`/`start` JSON response.

### Locking and budgets

Auto-GC must avoid multiple concurrent agent invocations doing expensive full-root cleanup at once.

Recommended behavior:

- Use a lightweight root-local lock file or equivalent cross-process guard.
- If the lock cannot be acquired immediately, skip auto-GC.
- Apply a scan budget and delete budget for auto-GC so a huge root cannot dominate `run`/`start` latency.
- Manual `gc` should not be silently budget-limited unless the user supplies a limit flag.

### Cleanup policy

Default auto-GC policy:

- retention: `30d`
- delete only terminal jobs older than retention
- skip `running`, `created`, unreadable/malformed state, and too-recent terminal jobs
- preserve existing post-delete absence check before reporting deletion

Manual `gc` adds optional pressure-based controls:

- `--max-jobs <N>` keeps the newest `N` terminal jobs and marks older terminal jobs as candidates.
- `--max-bytes <BYTES>` removes old terminal jobs until the root is at or below the target when possible.
- `--older-than <DURATION>` remains supported and defaults to `30d` when no other policy is supplied.

The implementation must make candidate reasons visible in dry-run/manual output so users can distinguish age, count, and byte-pressure cleanup.

## Response compatibility

Existing response fields must remain stable. New fields may be added as optional additions.

Manual `gc` should keep current fields:

- `root`
- `dry_run`
- `older_than`
- `older_than_source`
- `deleted`
- `skipped`
- `out_of_scope`
- `failed`
- `freed_bytes`
- `jobs`

New summary fields should be additive.

For `run` / `start`, auto-GC metadata should either be omitted by default or added only as optional fields that do not remove or rename existing inline output fields.

## Verification Strategy

- Unit tests cover planner logic without spawning jobs.
- Integration tests create real temporary job roots and execute the compiled CLI to prove filesystem side effects.
- Tests must include at least one stale terminal deletion, one active-job preservation case, one dry-run no-side-effect case, and one run/start JSON contract check.
