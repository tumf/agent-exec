## Implementation Tasks

- [ ] Update the JSON fixture in `tests/integration.rs` for `compression_modes_have_behavior_for_errors_logs_and_json` so the raw JSON is larger than the `object keys=2 [...]` shape summary. Completion condition: the test still asserts `compression.stdout` contains `object keys=2`, and the fixture cannot be suppressed by the expansion guard due to equal-or-larger compressed output. (verification: integration - `cargo test --test integration compression_modes_have_behavior_for_errors_logs_and_json -- --nocapture`)

- [ ] Add or adjust regression coverage for short JSON output that triggers the expansion guard under `run --compress json`. Completion condition: the regression asserts `compression.applied=false`, `compression.strategy` contains `expansion-guard`, `compression.stdout` is empty, and canonical raw `stdout` still contains the original short JSON. (verification: integration - `cargo test --test integration compression_json_expansion_guard -- --nocapture` or equivalent focused integration test in `tests/integration.rs`)

- [ ] Run repository verification commands after the test changes. Completion condition: formatting, clippy, and tests pass without changing production compression semantics. (verification: integration - `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --all` or `prek run -a`)

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate fix-ci-json-compression-fixture --archive-gate`

## Future Work

- Re-run GitHub Actions after the implementation commit is pushed to verify the Linux CI runner reproduces the local pass.
