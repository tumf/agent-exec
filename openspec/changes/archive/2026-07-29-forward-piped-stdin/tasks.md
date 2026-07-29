## Implementation Tasks

- [x] Resolve absent `run` stdin options to an implicit non-TTY stdin source while leaving TTY stdin and all `create` invocations explicit-only; keep `--stdin` and `--stdin-file` authoritative (verification: integration - `cargo test --test integration stdin`; verification-id: piped-stdin-tests).
- [x] Route implicit stdin through the existing bounded `stdin.bin` materialization and metadata path before supervisor launch, including the existing `stdin_too_large` error contract and cleanup semantics (verification: integration - `cargo test --test integration stdin`; verification-id: piped-stdin-tests).
- [x] Add integration regressions in `tests/integration.rs` for exact piped-byte forwarding, `meta.json.stdin_file`, explicit-source precedence, implicit size-limit failure, and null/TTY behavior where supported by the harness (verification: integration - `cargo test --test integration stdin` executes the compiled CLI and asserts child output plus persisted job evidence in `tests/integration.rs`; verification-id: piped-stdin-tests).
- [x] Update CLI help and maintained user guidance that describes stdin so the shorthand `producer | agent-exec run -- consumer`, explicit-source precedence, TTY behavior, and `create` exclusion are discoverable (verification: integration - `cargo test --test integration stdin` asserts help text sourced from `src/main.rs`, and repository review confirms updated stdin examples in maintained guidance; verification-id: piped-stdin-tests).
- [x] Run repository formatting, lint, and regression gates without weakening hooks or warnings (verification: integration - `prek run -a` executes the tracked hooks in `prek.toml` and exited 0 with trailing-whitespace, end-of-file-fixer, check-toml, check-yaml, `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`, and `cargo test --all` all reported Passed; verification-id: rust-quality-gates).

## Future Work

- Interactive stdin attachment and PTY support require a separate lifecycle and transport design.
- Other execution surfaces may adopt implicit stdin only through separate proposals with transport-specific safety analysis.

## Notes

- evidence: `prek run -a` exited 0 with every hook Passed (`cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`).
- evidence: `cargo test --test integration stdin` passes; the 11 stdin cases added by this change (4 unit in `src/run.rs`, 7 integration in `tests/integration.rs`) pass on every observed run.
- Pre-existing flaky tests, unrelated to this change: `compression_expansion_guard_applies_per_stream` and `compression_route_reports_specific_detected_kinds` in `tests/integration.rs` intermittently fail under full-suite parallelism. Across four full-suite runs on this branch the failure appeared twice, on a *different* test each time, and one run was fully green (268 passed). The base commit `d398ef9` likewise flaked, but in another suite (`test_cors_with_allow_origin` in `serve_integration`).
- Root cause of that flake is in untouched product code: the inline wait loop in `src/run.rs:803-812` breaks as soon as *either* `stdout.log` or `stderr.log` is non-empty, so `run` may intentionally return before the child has written both streams. Any test asserting on both `stdout` and `stderr` of a short-lived job races the child's writes. This change adds no code to that path; it only shifted libtest scheduling by adding 8 integration cases.
- Fixing that race is out of scope here: the early break is deliberate `run` semantics (see `wait_default_until_returns_non_terminal_for_long_running_job`), so it belongs to a separate proposal covering either the drain contract or the affected compression assertions.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate forward-piped-stdin --archive-gate`
