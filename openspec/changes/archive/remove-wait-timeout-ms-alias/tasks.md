## Implementation Tasks

- [x] 1. Remove the deprecated `--timeout-ms` alias from the `wait` clap definition in `src/main.rs` while preserving `--until`, `--forever`, and the default 30,000ms deadline semantics (verification: integration - `tests/integration.rs` exercises `wait`, `wait --until`, `wait --forever`, and `wait --timeout-ms` usage error behavior).
- [x] 2. Update `tests/integration.rs` so normative `wait` coverage uses only canonical spellings and add an explicit assertion that `agent-exec wait --timeout-ms ...` exits with clap usage error code 2 and empty stdout (verification: integration - `cargo test --test integration wait_` and the new invalid-flag test).
- [x] 3. Update `README.md` and `skills/agent-exec/SKILL.md` so user-facing guidance documents only `--until` / `--forever` for wait deadlines and no longer describe `--timeout-ms` as supported behavior (verification: manual - search for `timeout-ms` in those files returns no wait guidance; docs show `--until` instead).
- [x] 4. Run repository verification for the CLI-contract change: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` (or `prek run -a` if preferred for CI parity) (verification: command outputs show success).

## Future Work

- Review whether any external automation or downstream wrappers still rely on `wait --timeout-ms` before the next release notes are drafted.
