---
change_type: implementation
priority: high
dependencies: []
references:
  - AGENTS.md
  - README.md
  - src/gc.rs
  - src/run.rs
  - src/start.rs
  - src/schema.rs
  - src/jobstore.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-jobstore/spec.md
---

# Add bounded auto-GC and jobstore cleanup controls

**Change Type**: implementation

## Premise / Context

- The user reported that `agent-exec` does not keep the jobs directory organized.
- The default jobs root resolves to `~/.local/share/agent-exec/jobs`, and current storage is a flat `<root>/<job_id>/` directory layout.
- The current implementation has manual cleanup via `gc` and cwd-scoped `delete --all`, but `run` and `start` do not trigger any cleanup automatically.
- Repository guidance treats `run` / `start` returning inline output after a short default wait as a core concept; cleanup must not turn these commands into extra round-trip workflows or unbounded scans.
- Existing specs require JSON-only stdout, safe deletion observability, terminal-only GC behavior, and backward compatibility for existing job directories.

## Problem / Context

`agent-exec` creates one directory per job under a flat jobs root. Heavy use leaves thousands of terminal job directories in the default root unless the user remembers to run `agent-exec gc` manually. This creates operational noise for agents, slows root scans, and makes the jobstore feel unmanaged even though terminal jobs are no longer useful after a retention window.

Manual `gc` is useful but insufficient as the primary hygiene mechanism because the tool is designed for low-friction agent use. Cleanup should happen safely and opportunistically without requiring the user or agent to add a second command after routine `run` or `start` calls.

## Proposed Solution

Add bounded automatic jobstore cleanup and improve manual GC controls while preserving the current flat job directory layout.

1. Add a shared GC evaluation engine used by both manual `gc` and automatic cleanup.
2. Run opportunistic auto-GC after successful job creation/launch paths for `run` and `start`, using the same terminal-only safety rules as manual `gc`.
3. Keep auto-GC bounded by a small scan/deletion budget so it cannot dominate the default inline-observation response path.
4. Add explicit opt-out and tuning knobs for auto-GC.
5. Extend manual `gc` with count/size based cleanup controls and summary fields that make jobs-root health observable.
6. Preserve existing `<root>/<job_id>/` layout and job lookup compatibility.

## Acceptance Criteria

- `agent-exec run` and `agent-exec start` opportunistically remove old terminal job directories according to a default 30-day retention policy without requiring a separate `gc` invocation.
- Auto-GC never deletes `running` or `created` jobs and never reports deletion unless the job directory is absent at command completion.
- Auto-GC is bounded and best-effort: lock contention, unreadable jobs, or budget exhaustion must not fail the parent `run` / `start` command.
- Users can disable auto-GC per invocation and configure its retention/budget behavior through CLI or config-backed settings.
- Manual `agent-exec gc` supports retention, max terminal job count, and max root byte controls with dry-run support.
- `gc` JSON output includes summary information sufficient to understand root size, scanned entries, terminal/running/created/unknown counts, deletion candidates, and skipped/failed outcomes.
- Existing JSON-only stdout, `run` / `start` inline output fields, job ID layout, prefix lookup, and ULID compatibility remain intact.

## Explicit Completion Conditions

- `src/gc.rs` exposes shared internal evaluation/execution code used by manual `gc` and by auto-GC call sites.
- `src/run.rs` and `src/start.rs` invoke best-effort bounded auto-GC on successful launch paths without changing required run/start response fields.
- CLI/config parsing accepts documented auto-GC and manual GC knobs, rejects invalid values with structured/usage errors as appropriate, and preserves defaults for existing invocations.
- `src/schema.rs` represents any new GC summary fields and optional run/start auto-GC metadata without removing existing response fields.
- Integration tests in `tests/integration.rs` prove terminal jobs are deleted, non-terminal jobs are preserved, dry-run has no side effects, budget/opt-out paths are honored, and `run` / `start` still emit valid JSON-only responses.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Changing the job directory layout from `<root>/<job_id>/` to date/cwd sharded directories.
- Migrating existing job directories into a new hierarchy.
- Deleting or compacting logs for jobs that are still `running` or `created`.
- Introducing background daemons, scheduled tasks, or external cleanup services.
- Changing `list` default cwd filtering behavior.
