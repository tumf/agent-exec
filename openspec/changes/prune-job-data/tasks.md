## Implementation Tasks

- [x] Task 1: Add the `gc` CLI surface and JSON response wiring in `src/main.rs` and `src/schema.rs`, including the default 30-day retention when `--older-than` is omitted (verification: `src/main.rs` exposes `Command::Gc` with optional `older_than`; `src/schema.rs` defines the `gc` response envelope including `older_than_source`)
- [x] Task 2: Implement root-wide GC traversal and candidate evaluation in a dedicated module such as `src/gc.rs`, using terminal-state-only deletion and `finished_at -> updated_at` cutoff logic (verification: `src/gc.rs` evaluates `exited|killed|failed`, never deletes `running`, and skips unreadable/incomplete jobs with explicit reasons)
- [x] Task 3: Add jobstore helpers needed for recursive size calculation and directory removal without changing existing `run/status/tail/wait/kill` behavior (verification: `src/jobstore.rs` or the new GC module contains reusable helpers for byte accounting and safe directory deletion)
- [x] Task 4: Add integration tests for default-30d mode, custom delete mode, dry-run mode, and safety cases in `tests/integration.rs` (verification: `cargo test --test integration gc_uses_default_30d_window gc_deletes_only_terminal_jobs gc_dry_run_preserves_directories gc_skips_jobs_without_gc_timestamp` passes)
- [x] Task 5: Update `README.md` with `gc` usage examples and retention semantics (verification: `README.md` documents `agent-exec gc --older-than <duration>` and `--dry-run`)

## Future Work

- Add count-based retention or hybrid retention only after age-based GC proves sufficient in real usage.
- Consider config-file defaults for retention windows only after the explicit CLI flow is established.
