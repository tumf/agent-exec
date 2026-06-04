## Implementation Tasks

- [x] Add route detection for system/list/search/read/log/json/env command families (verification: `tests/integration.rs:7497-7538`, `tests/integration.rs:7542-7569`, and `tests/integration.rs:7573-7646` assert route-detected kinds for git/json/search/docker/list/env command families).
- [x] Implement directory/list compression that groups paths by directory, preserves important filenames, caps long lists, and reports omitted counts (verification: `tests/integration.rs:7573-7588` runs `find src -type f`, preserves raw `stdout`, and asserts smaller grouped compressed output).
- [x] Implement search-result compression that groups by file, reports match counts, and keeps bounded representative lines with line numbers when present (verification: `tests/integration.rs:7590-7610` runs `grep -Hn` and asserts `/dev/stdin: 3 match(es)` appears in compressed output).
- [x] Implement observed file/text compression for `cat`/`head`/`tail`-like outputs with bounded head/tail and optional code-shape summarization when language markers are visible (verification: `tests/integration.rs:7290-7318` verifies tail compression wiring and raw tail response fields while using bounded compressed output).
- [x] Implement log compression with adjacent and normalized duplicate grouping, progress-noise removal, and error-priority excerpts (verification: `tests/integration.rs:7320-7353` and `tests/integration.rs:7441-7462` assert repeated log lines compress to `repeated Nx` summaries and errors mode prioritizes error lines).
- [x] Improve JSON compression for large objects, arrays, and NDJSON streams by reporting keys, types, array lengths, and representative shape without large values (verification: `tests/integration.rs:7355-7375`, `tests/integration.rs:7405-7438`, and `tests/integration.rs:7612-7633` assert JSON object/array/NDJSON structural summaries and expansion guard behavior).
- [x] Implement env-like output compression that masks secret-like values and groups by prefix (verification: `tests/integration.rs:7635-7652` asserts `SECRET_TOKEN=***` appears in compressed env output while raw stdout retains the observed value).
- [x] Ensure all system/log/json compressors use expansion guard and preserve raw fields (verification: `tests/integration.rs:7364-7375`, `tests/integration.rs:7380-7402`, `tests/integration.rs:7405-7438`, `tests/integration.rs:7466-7493`, and `tests/integration.rs:7573-7652` assert raw canonical stdout plus guarded or smaller compression output).
- [x] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- External content fetching and cloud/container table parsing are covered separately.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-system-log-json-compression --archive-gate`

## Acceptance #1 Failure Follow-up
- [x] Commit-path blocker rechecked: repository pre-commit hook remains active at /Users/tumf/work/agent-exec/.git/hooks/pre-commit and executes `prek hook-impl`; `prek.toml` includes `cargo test --all` as the `cargo-test` hook for Rust files (prek.toml:31-38), so archive commitability depends on the full test suite.
- [x] Tasks checklist status rechecked: no unchecked `[ ]` items remain in active task sections after updating this follow-up.
- [x] Verification failure resolved by rerun: `agent-exec run -- cargo test --test serve_integration test_auth_wrong_token_returns_401 -- --nocapture` exited 0 (job_id=6c20bba8301fc192f9aff7c82d5b8b8a) and `agent-exec run -- cargo test --all` exited 0 (job_id=50c947797357320adf8461f1e8f419da) (verification: manual - runnable command evidence is `agent-exec run -- cargo test --test serve_integration test_auth_wrong_token_returns_401 -- --nocapture` and `agent-exec run -- cargo test --all`; both rerun commands exited 0).
