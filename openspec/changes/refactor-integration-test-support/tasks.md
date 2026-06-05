## Implementation Tasks

- [x] Extract reusable binary invocation and isolated-root support from `tests/integration.rs` into a shared test support module (verification: integration - `tests/support/mod.rs` defines `TestHarness::run`, `run_cmd_with_root`, and `run_cmd_with_root_and_cwd`; `tests/integration.rs` imports and calls these helpers for normal and cwd-scoped command execution).
- [x] Extract reusable assertion helpers for JSON envelope checks, usage errors, and raw stdout/stderr diagnostics (verification: integration - `tests/support/mod.rs` defines `assert_envelope`, `assert_usage_error`, `assert_common_fields`, and raw process helpers; usage-error tests in `tests/integration.rs` call `assert_usage_error` and preserve exit-code-2/empty-stdout assertions).
- [x] Route stdin-based test execution and global/subcommand root variants through the shared support without changing assertions (verification: integration - `tests/support/mod.rs` defines `run_cmd_with_root_and_stdin`, `run_cmd_with_global_root_flag`, and `run_cmd_with_subcommand_root_flag`; stdin and root precedence tests in `tests/integration.rs` call those shared helpers).
- [x] Reuse shared support in at least one additional integration suite or clearly separated test section when practical (verification: integration - `tests/serve_integration.rs` or another integration module compiles with shared helpers where behavior overlaps).
- [x] Run formatting, linting, and tests after refactor (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).

## Final Validation

Expected OpenSpec archive gate: `cflx openspec validate refactor-integration-test-support --archive-gate`.

## Future Work

- Further splitting large behavioral test sections by command can be proposed separately if this support layer proves stable.

## Acceptance #1 Failure Follow-up
- [x] Add concrete repository-verifiable evidence to implementation task verification notes (verification: manual - lines 3-5 cite `tests/support/mod.rs`, `tests/integration.rs`, helper names, and preserved assertion paths).
