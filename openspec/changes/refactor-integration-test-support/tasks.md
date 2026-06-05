## Implementation Tasks

- [x] Extract reusable binary invocation and isolated-root support from `tests/integration.rs` into a shared test support module (verification: integration - existing tests compile and call the shared support for normal command execution).
- [x] Extract reusable assertion helpers for JSON envelope checks, usage errors, and raw stdout/stderr diagnostics (verification: integration - usage-error tests still assert exit code 2 and empty stdout through shared helpers).
- [x] Route stdin-based test execution and global/subcommand root variants through the shared support without changing assertions (verification: integration - existing stdin and root precedence tests continue to pass).
- [x] Reuse shared support in at least one additional integration suite or clearly separated test section when practical (verification: integration - `tests/serve_integration.rs` or another integration module compiles with shared helpers where behavior overlaps).
- [x] Run formatting, linting, and tests after refactor (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).

## Final Validation

Expected OpenSpec archive gate: `cflx openspec validate refactor-integration-test-support --archive-gate`.

## Future Work

- Further splitting large behavioral test sections by command can be proposed separately if this support layer proves stable.
