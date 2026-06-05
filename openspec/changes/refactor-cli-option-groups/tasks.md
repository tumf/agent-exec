## Implementation Tasks

- [x] Extract shared definition-time option mapping for `create` and `run`, preserving persisted metadata inputs and public clap annotations (verification: integration - add or retain tests in `tests/integration.rs` that compare representative `meta.json` fields for `create` and `run`).
- [x] Extract shared auto-GC option mapping for command dispatch paths that launch or restart jobs (verification: integration - `src/main.rs` `AutoGcOptions`; `cargo test --all`).
- [x] Extract shared inline observation and compression option mapping for `run`, `start`, and `restart` dispatch paths (verification: integration - `src/main.rs` `InlineObservationOptions`; `cargo test --all`).
- [x] Preserve usage-error behavior for conflicting flags and command-specific flags (verification: integration - `tests/integration.rs` usage-error tests; `cargo test --all`).
- [x] Run formatting, linting, and tests after refactor (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).

## Final Validation

Expected OpenSpec archive gate: `cflx openspec validate refactor-cli-option-groups --archive-gate`.

## Future Work

- Broader public CLI redesign is intentionally excluded; this change only reduces internal duplication.
