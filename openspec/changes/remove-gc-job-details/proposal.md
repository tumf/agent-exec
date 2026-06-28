---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/gc.rs
  - src/schema.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
---

# Remove per-job details from gc output

**Change Type**: implementation

## Problem / Context

`agent-exec gc` currently returns a `jobs` array containing one entry per scanned or selected job. This makes cleanup responses noisy and expensive when many job directories exist, while the command already exposes aggregate counters such as `deleted`, `skipped`, `out_of_scope`, `failed`, `freed_bytes`, `scanned_dirs`, and `candidate_count`.

The requested behavior is to stop emitting per-job details from `gc` and keep the response summary-only.

## Proposed Solution

Remove `jobs` from the public `gc` response schema and from manual `gc` output. Preserve the existing aggregate counters and deletion behavior. Update implementation and integration tests so `gc` verification relies on summary counters and filesystem side effects rather than per-job `job_id` entries.

## Acceptance Criteria

- `agent-exec gc` returns a single JSON response with the existing summary fields and no `jobs` field.
- `agent-exec gc --dry-run` returns summary counts and `freed_bytes` / `candidate_count` without deleting directories and without a `jobs` field.
- `gc` still deletes only eligible terminal jobs and preserves running / created / out-of-scope jobs.
- `gc` still verifies deletion before incrementing `deleted` and `freed_bytes`.
- Existing stdout JSON-only and error boundary contracts remain unchanged.

## Explicit Completion Conditions

- `src/schema.rs` no longer exposes `GcData.jobs` or a `GcJobResult` type used only by `gc`.
- `src/gc.rs` no longer builds or returns per-job result vectors for `gc`, while retaining existing counters and filesystem behavior.
- `tests/integration.rs` has regression coverage asserting `gc` responses omit `jobs` and still report correct summary counts for delete, dry-run, skipped, max-jobs, max-bytes, and unreadable-state paths.
- Rust formatting, clippy, and tests pass using the repository commands from `AGENTS.md`.

## Out of Scope

- Changing `delete` output shape.
- Adding a replacement verbose flag or detailed GC report mode.
- Changing GC retention, max-jobs, max-bytes, auto-GC, or filesystem deletion policy semantics.
