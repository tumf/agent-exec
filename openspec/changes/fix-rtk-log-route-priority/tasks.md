## Implementation Tasks

- [x] Add or expose a normalized repeated-log detector for route classification that recognizes timestamp-varied repeated messages, including messages containing `ERROR` (verification: unit - `src/compress/util.rs::tests::repeated_normalized_log_detector_recognizes_timestamp_varied_errors` and `src/compress/route.rs::tests::classifies_timestamp_normalized_repeated_error_logs_before_errors` cover repeated `2026-01-01T00:00:%02dZ ERROR retry failed` routing to `DetectedKind::Logs`).
- [x] Adjust output-shape fallback priority in `src/compress/route.rs` so normalized repeated logs are classified before generic `looks_like_error_output` (verification: unit - `src/compress/route.rs::tests::classifies_timestamp_normalized_repeated_error_logs_before_errors`, `src/compress/route.rs::tests::classifies_stderr_timestamp_normalized_repeated_error_logs_before_errors`, and `src/compress/route.rs::tests::classifies_single_error_as_errors`).
- [x] Add an integration regression test using `agent-exec run --rtk route -- python3 -c 'for i in range(80): print("2026-01-01T00:00:%02dZ ERROR retry failed" % (i%10))'` or an equivalent fixture command (verification: integration - `tests/integration.rs::compression_route_classifies_timestamp_varied_repeated_error_logs_as_logs` asserts `compression.detected_kind = "logs"`, `dedupe-normalized-log-lines`, and compressed bytes significantly below raw bytes).
- [x] Preserve existing route behavior for exact adjacent repeated lines, JSON output, command-family routes, and single error output (verification: unit/integration - `src/compress/route.rs::tests::classifies_repeated_logs`, `src/compress/route.rs::tests::classifies_json_output`, `src/compress/route.rs::tests::classifies_single_error_as_errors`, `tests/integration.rs::compression_route_reports_specific_detected_kinds`, and `tests/integration.rs::compression_route_preserves_single_error_as_errors`).
- [x] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` or the agreed compression-focused subset followed by full CI).

## Future Work

- Tune the logs compressor output to reduce repeated `ERROR excerpt` lines further if future demos show insufficient compression after routing is fixed.

## Final Validation

Expected archive gate: `cflx openspec validate fix-rtk-log-route-priority --archive-gate`

## Acceptance #1 Failure Follow-up
- [x] Update completed task verification notes with repository-verifiable evidence so `cflx openspec validate fix-rtk-log-route-priority --archive-gate` can pass (verification: manual - task verification notes now cite `src/compress/util.rs`, `src/compress/route.rs`, `tests/integration.rs`, and exact runnable test names; verification command `agent-exec run -- zsh -lc 'cargo test repeated_normalized_log_detector_recognizes_timestamp_varied_errors && cargo test classifies_timestamp_normalized_repeated_error_logs_before_errors && cargo test classifies_stderr_timestamp_normalized_repeated_error_logs_before_errors && cargo test --test integration compression_route_classifies_timestamp_varied_repeated_error_logs_as_logs && cargo test --test integration compression_route_preserves_single_error_as_errors'` passed in job `eb74c071bb432e04cc09e0ff796eccb5`).
