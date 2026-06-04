## Implementation Tasks

- [ ] Separate command-family classification from output-shape fallback classification in `src/compress/route.rs` without changing priority order (verification: unit - existing route classification tests plus representative new tests for JSON-before-logs and logs-before-errors ordering).
- [ ] Split summarizer responsibilities in `src/compress/generic.rs` into smaller family-specific helpers or modules while preserving `compress_kind` behavior (verification: unit - existing compression tests pass for Git, tests/errors, JSON, search, env, and table outputs).
- [ ] Preserve safety utilities such as `fallback_if_empty` and `guard_expansion` as final response-building safeguards (verification: unit - existing expansion guard tests still prove oversized candidates are suppressed).
- [ ] Add boundary-focused regression tests for routed compression that would fail if classification and summarization are accidentally mismatched (verification: integration - `agent-exec run --rtk route -- <representative command>` yields the same `compression.detected_kind` and meaningful compressed view).
- [ ] Run formatting, linting, and tests after refactor (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).

## Final Validation

Expected OpenSpec archive gate: `cflx openspec validate refactor-compression-routing --archive-gate`.

## Future Work

- Adding additional command families should be proposed separately after this structure is in place.
