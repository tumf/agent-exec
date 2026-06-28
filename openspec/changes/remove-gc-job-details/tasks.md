## Implementation Tasks

- [ ] Remove `GcJobResult` struct from `src/schema.rs` and the `jobs: Vec<GcJobResult>` field from `GcData` (verification: unit - `cargo build` compiles without `GcJobResult` usage errors)
- [ ] Remove per-job `results` vector construction and population from `src/gc.rs` `run_gc`, keeping only aggregate counter logic (verification: unit - `cargo build`)
- [ ] Remove `GcJobResult` from the `gc.rs` import in `src/schema.rs` usage (verification: unit - `cargo clippy --all-targets --all-features -- -D warnings`)
- [ ] Update `assert_gc_envelope` in `tests/integration.rs` to assert `jobs` is absent from the gc response (verification: integration - `cargo test --test integration gc_empty_root_returns_ok`)
- [ ] Rewrite `gc_deletes_only_terminal_jobs` test to verify via summary counters (`deleted`, `skipped`, `out_of_scope`) and filesystem checks instead of per-job `jobs` array (verification: integration - `cargo test --test integration gc_deletes_only_terminal_jobs`)
- [ ] Rewrite `gc_dry_run_preserves_directories` test to verify via `candidate_count`, `freed_bytes`, and filesystem checks (verification: integration - `cargo test --test integration gc_dry_run_preserves_directories`)
- [ ] Rewrite `gc_supports_max_jobs_policy` test to use `candidate_count` instead of filtering `jobs` array (verification: integration - `cargo test --test integration gc_supports_max_jobs_policy`)
- [ ] Rewrite `gc_supports_max_bytes_policy` test to use summary counters instead of filtering `jobs` array reasons (verification: integration - `cargo test --test integration gc_supports_max_bytes_policy`)
- [ ] Rewrite `gc_skips_unreadable_state` test to verify via `skipped` counter instead of per-job entry (verification: integration - `cargo test --test integration gc_skips_unreadable_state`)
- [ ] Verify `gc_deleted_action_implies_directory_absent_and_categorises_skips` test still passes with summary-only assertions (verification: integration - `cargo test --test integration gc_deleted_action_implies_directory_absent_and_categorises_skips`)
- [ ] Run full CI parity check (verification: integration - `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all`)

## Future Work

- Consider adding `--verbose` flag to re-introduce per-job details if users request it.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate remove-gc-job-details --archive-gate`
