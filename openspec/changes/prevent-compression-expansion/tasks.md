## Implementation Tasks

- [x] Add a shared expansion guard to compression result generation. Completion condition: `src/compress.rs` or equivalent code compares generated compressed stream text against the raw observed stream text and suppresses compressed text when it would be larger than or equal to raw. (verification: unit - add `src/compress.rs` tests that cover smaller, equal-size, and larger compressed candidate cases and run `cargo test compress`)

- [x] Return bounded fallback compression metadata when the guard triggers. Completion condition: guarded responses keep `compression` present for non-`off` modes, set `applied=false`, avoid large compressed `stdout`/`stderr`, and include an explicit strategy/reason such as `expansion-guard`. (verification: integration - `tests/integration.rs` asserts the fallback payload is shorter than the raw observed payload, includes the guard reason, and passes with `cargo test --test integration compression_expansion_guard`)

- [x] Preserve existing raw observation and `off` mode contracts. Completion condition: canonical `stdout`/`stderr`, range fields, total byte fields, and `encoding` remain raw and unchanged by the guard; `--compress off` still omits `compression`. (verification: integration - `tests/integration.rs` run/tail compression tests assert raw fields and `off` omission after guard implementation)

- [x] Cover the observed tail expansion risk with an integration regression test. Completion condition: an integration test tails a job whose command output is JSON/NDJSON-like enough to make naive compression expand, and verifies `compression.applied=false` rather than a larger `compression.stdout`. (verification: integration - `cargo test --test integration compression_expansion_guard` fails against the pre-guard implementation)

- [x] Ensure useful compression still applies when it is smaller. Completion condition: repeated-line log output or error-focused output still returns `compression.applied=true` with compact text when the compressed text is smaller than raw. (verification: integration - `tests/integration.rs` includes or updates a repeated/noisy output case and passes with `cargo test --test integration compression_smaller_output_still_applies`)

- [x] Run final repository verification. Completion condition: formatting, linting, and test suite pass locally. (verification: integration - run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`)

## Future Work

- Consider exposing a separate metric for skipped compression attempts if users later need diagnostics.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate prevent-compression-expansion --archive-gate`.
