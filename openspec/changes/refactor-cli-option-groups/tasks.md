## Implementation Tasks

- [ ] Extract shared definition-time option mapping for `create` and `run`, preserving persisted metadata inputs and public clap annotations (verification: integration - add or retain tests in `tests/integration.rs` that compare representative `meta.json` fields for `create` and `run`).
- [ ] Extract shared auto-GC option mapping for command dispatch paths that launch or restart jobs (verification: integration - existing auto-GC tests still pass and at least one path exercises the shared mapping).
- [ ] Extract shared inline observation and compression option mapping for `run`, `start`, and `restart` dispatch paths (verification: integration - existing inline output and compression tests still pass for all supported commands).
- [ ] Preserve usage-error behavior for conflicting flags and command-specific flags (verification: integration - existing clap usage-error tests continue to assert exit code 2 and empty stdout).
- [ ] Run formatting, linting, and tests after refactor (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).

## Final Validation

Expected OpenSpec archive gate: `cflx openspec validate refactor-cli-option-groups --archive-gate`.

## Future Work

- Broader public CLI redesign is intentionally excluded; this change only reduces internal duplication.
