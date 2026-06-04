## Implementation Tasks

- [ ] Add detection for `cargo test`, `cargo build`, `cargo check`, `cargo clippy`, and common generic test invocations (`npm test`, `pnpm test`, `pytest`, `vitest`, `jest`, `go test`) (verification: unit - classifier tests produce `cargo-test`, `cargo-build`, `test-runner`, or specific runner kinds).
- [ ] Implement Rust compiler diagnostic block extraction preserving `error[...]`, `warning[...]`, file:line, primary message, note/help blocks, and bounded source snippets (verification: unit - cargo build/clippy/check fixtures retain diagnostic essence and remove compile progress noise).
- [ ] Implement cargo test summarization that retains failure names and failure details while aggregating passing tests and final result counts (verification: integration - `agent-exec run --rtk route -- cargo test <fixture>` or synthetic fixture command produces smaller `compression.stdout` with failure detail).
- [ ] Implement generic test-output compression for common PASS/FAIL/SKIP patterns and bounded stack traces (verification: unit - pytest/vitest/jest/go-test-like fixtures preserve failures only and summarize pass counts).
- [ ] Ensure panic/backtrace output keeps top relevant frames and assertion/error text while bounding repetitive frames (verification: unit - panic fixture is smaller and still contains panic message and location).
- [ ] Ensure small successful outputs are guarded rather than expanded (verification: integration - small `cargo --version`-like output under route does not embed larger compression text).
- [ ] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Runner-specific JSON/NDJSON parsing for JS/Python/Go is covered by `add-rtk-js-python-go-compression`.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-rust-test-compression --archive-gate`
