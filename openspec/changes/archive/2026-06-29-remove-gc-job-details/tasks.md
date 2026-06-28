## Implementation Tasks

- [x] Remove `GcJobResult` struct from `src/schema.rs` and the `jobs: Vec<GcJobResult>` field from `GcData` (verification: unit - `cargo build` compiles without `GcJobResult` usage errors)
- [x] Remove per-job `results` vector construction and population from `src/gc.rs` `run_gc`, keeping only aggregate counter logic (verification: unit - `cargo build`)
- [x] Remove any stale `GcJobResult` imports/usages from `src/schema.rs` and `src/gc.rs` (verification: unit - `src/schema.rs`, `src/gc.rs`, `cargo clippy --all-targets --all-features -- -D warnings`)
- [x] Update `assert_gc_envelope` in `tests/integration.rs` to assert `jobs` is absent from the gc response (verification: integration - `cargo test --test integration gc_empty_root_returns_ok`)
- [x] Rewrite `gc_deletes_only_terminal_jobs` test to verify via summary counters (`deleted`, `skipped`, `out_of_scope`) and filesystem checks instead of per-job `jobs` array (verification: integration - `cargo test --test integration gc_deletes_only_terminal_jobs`)
- [x] Rewrite `gc_dry_run_preserves_directories` test to verify via `candidate_count`, `freed_bytes`, and filesystem checks (verification: integration - `cargo test --test integration gc_dry_run_preserves_directories`)
- [x] Rewrite `gc_supports_max_jobs_policy` test to use `candidate_count` instead of filtering `jobs` array (verification: integration - `cargo test --test integration gc_supports_max_jobs_policy`)
- [x] Rewrite `gc_supports_max_bytes_policy` test to use summary counters instead of filtering `jobs` array reasons (verification: integration - `cargo test --test integration gc_supports_max_bytes_policy`)
- [x] Rewrite `gc_skips_unreadable_state` test to verify via `skipped` counter instead of per-job entry (verification: integration - `cargo test --test integration gc_skips_unreadable_state`)
- [x] Verify `gc_deleted_action_implies_directory_absent_and_categorises_skips` test still passes with summary-only assertions (verification: integration - `cargo test --test integration gc_deleted_action_implies_directory_absent_and_categorises_skips`)
- [x] Run full CI parity check (verification: integration - `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all`)

## Future Work

- Consider adding `--verbose` flag to re-introduce per-job details if users request it.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate remove-gc-job-details --archive-gate`
