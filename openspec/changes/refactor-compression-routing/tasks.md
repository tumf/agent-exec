## Implementation Tasks

- [x] Separate command-family classification from output-shape fallback classification in `src/compress/route.rs` without changing priority order (verification: unit - `cargo test output_shape_priority`; tests in `src/compress/route.rs` cover JSON-before-logs, logs-before-errors, and command-family-before-output-shape ordering).
- [x] Split summarizer responsibilities in `src/compress/generic.rs` into smaller family-specific helpers or modules while preserving `compress_kind` behavior (verification: unit - `cargo test compress::`; tests in `src/compress/generic.rs`, `src/compress/language.rs`, and `src/compress/util.rs` cover Git, tests/errors, JSON, search, env, and table outputs).
- [x] Preserve safety utilities such as `fallback_if_empty` and `guard_expansion` as final response-building safeguards (verification: unit - `cargo test compress::`; `src/compress/generic.rs` still finalizes via `fallback_if_empty`, `src/compress/mod.rs` still applies `guard_expansion`, and `src/compress/util.rs` expansion guard tests passed).
- [x] Add boundary-focused regression tests for routed compression that would fail if classification and summarization are accidentally mismatched (verification: integration - `cargo test compression_route_boundary_pairs_output_shape_with_matching_summarizer`; test in `tests/integration.rs` verifies `--rtk route` JSON and repeated-log commands produce matching `compression.detected_kind` and compressed summaries).
- [x] Run formatting, linting, and tests after refactor (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).

## Final Validation

Expected OpenSpec archive gate: `cflx openspec validate refactor-compression-routing --archive-gate`.

## Future Work

- Adding additional command families should be proposed separately after this structure is in place.
