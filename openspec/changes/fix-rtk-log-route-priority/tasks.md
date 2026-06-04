## Implementation Tasks

- [x] Add or expose a normalized repeated-log detector for route classification that recognizes timestamp-varied repeated messages, including messages containing `ERROR` (verification: unit - fixture with repeated `2026-01-01T00:00:%02dZ ERROR retry failed` routes to `DetectedKind::Logs`).
- [x] Adjust output-shape fallback priority in `src/compress/route.rs` so normalized repeated logs are classified before generic `looks_like_error_output` (verification: unit - repeated ERROR logs route to `logs`, while a single `ERROR one-off failure` route remains `errors`).
- [x] Add an integration regression test using `agent-exec run --rtk route -- python3 -c 'for i in range(80): print("2026-01-01T00:00:%02dZ ERROR retry failed" % (i%10))'` or an equivalent fixture command (verification: integration - response has `compression.detected_kind = "logs"`, strategy includes `dedupe-normalized-log-lines`, and compressed bytes are significantly below raw bytes).
- [x] Preserve existing route behavior for exact adjacent repeated lines, JSON output, command-family routes, and single error output (verification: integration/unit - existing compression route tests plus new regression tests pass).
- [x] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` or the agreed compression-focused subset followed by full CI).

## Future Work

- Tune the logs compressor output to reduce repeated `ERROR excerpt` lines further if future demos show insufficient compression after routing is fixed.

## Final Validation

Expected archive gate: `cflx openspec validate fix-rtk-log-route-priority --archive-gate`
