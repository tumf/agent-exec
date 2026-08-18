## Implementation Tasks

- [x] Add the status fields and presence rules defined in the spec, including separate live `elapsed_ms` and persisted `duration_ms`, `signal`, and `logs_drained`; construct them without exposing environment values, stdin content, or notification secrets (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [x] Extract a shared `Option<bool>` PID-liveness helper for `list`, `delete`, and `status`; preserve list/delete unsupported-platform fallbacks, avoid probing terminal jobs, and keep status read-only (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [x] Report canonical log paths and byte totals using file metadata only; add a large-log fixture proving status does not read log contents and missing/unreadable logs yield zero totals (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [x] Update schema version `0.3` across source, checked-in JSON Schema, changelog, tests, README, and bundled skill references; repair the existing StatusResponse schema drift and document the global-version effect on completion/output-match event envelopes (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [x] Add integration tests named with the `status_` prefix. At minimum include `status_reports_process_alive_for_live_pid`, `status_reports_process_alive_false_for_dead_pid_fixture`, `status_omits_process_alive_for_terminal_job`, `status_terminal_reports_persisted_duration_ms`, `status_reports_signal_and_logs_drained`, `status_reports_log_paths_and_byte_totals`, `status_missing_logs_report_zero_bytes`, `status_large_logs_use_bounded_size_observation`, `status_omits_env_values_and_stdin_content`, `status_does_not_rewrite_state_json`, and `status_response_validates_against_checked_in_schema`; zero matched tests do not constitute evidence (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)

## Notes

- Shared PID probe lives in `src/process.rs` as `pid_liveness(pid) -> Option<bool>`. `list` maps `None` to `true` (optimistic presentation) and `delete` maps `None` to `false` (do not block removal), preserving both pre-existing unsupported-platform fallbacks; `status` omits the field.
- `elapsed_ms` needs an RFC 3339 reader, so `src/run.rs` gained `parse_rfc3339_secs` next to the existing `format_rfc3339`, covered by unit tests (round trip, tolerated UTC spellings, malformed/non-UTC rejection).
- Unit evidence: `src/status.rs` presence-rule helpers (`resolve_process_alive`, `resolve_elapsed_ms`) and `src/run.rs` timestamp parsing are pure-logic tests with no external boundary.
- Integration evidence: `cargo test --test integration status_` -> 18 passed; 0 failed (11 new `status_`-prefixed tests plus the pre-existing `status_` suite).
- Repository-wide evidence: `make check` (fmt-check, clippy `-D warnings`, `cargo test --all`) -> "All checks passed!".
- Site docs (`site/docs/output-contracts.html`, `site/docs/quick-start.html`) also carried literal `schema_version` examples and were moved to `0.3` for consistency.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate enrich-status-diagnostics --archive-gate`.

Conflux acceptance must also run `make check`; it is not delegated to `prek.toml` because those hooks are staged-file scoped.
