## Implementation Tasks

- [x] Add detection for `cargo test`, `cargo build`, `cargo check`, `cargo clippy`, and common generic test invocations (`npm test`, `pnpm test`, `pytest`, `vitest`, `jest`, `go test`) (verification: unit - `src/compress.rs::tests::classifier_detects_rust_and_common_test_commands` via `cargo test compress::tests`).
- [x] Implement Rust compiler diagnostic block extraction preserving `error[...]`, `warning[...]`, file:line, primary message, note/help blocks, and bounded source snippets (verification: unit - `src/compress.rs::tests::rust_diagnostics_keep_essence_and_drop_progress` via `cargo test compress::tests`).
- [x] Implement cargo test summarization that retains failure names and failure details while aggregating passing tests and final result counts (verification: integration - `tests/integration.rs::compression_cargo_test_synthetic_fixture_keeps_failure_detail` via `cargo test --test integration compression_cargo_test_synthetic_fixture_keeps_failure_detail`).
- [x] Implement generic test-output compression for common PASS/FAIL/SKIP patterns and bounded stack traces (verification: unit - `src/compress.rs::tests::generic_test_compression_preserves_failures_only` via `cargo test compress::tests`).
- [x] Ensure panic/backtrace output keeps top relevant frames and assertion/error text while bounding repetitive frames (verification: unit - `src/compress.rs::tests::panic_backtrace_is_bounded` via `cargo test compress::tests`).
- [x] Ensure small successful outputs are guarded rather than expanded (verification: integration - `tests/integration.rs::compression_route_small_cargo_version_like_output_is_guarded` via `cargo test --test integration compression_route_small_cargo_version_like_output_is_guarded`).
- [x] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Runner-specific JSON/NDJSON parsing for JS/Python/Go is covered by `add-rtk-js-python-go-compression`.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-rust-test-compression --archive-gate`
