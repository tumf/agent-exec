## Implementation Tasks

- [ ] Add the status fields and presence rules defined in the spec, including separate live `elapsed_ms` and persisted `duration_ms`, `signal`, and `logs_drained`; construct them without exposing environment values, stdin content, or notification secrets (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [ ] Extract a shared `Option<bool>` PID-liveness helper for `list`, `delete`, and `status`; preserve list/delete unsupported-platform fallbacks, avoid probing terminal jobs, and keep status read-only (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [ ] Report canonical log paths and byte totals using file metadata only; add a large-log fixture proving status does not read log contents and missing/unreadable logs yield zero totals (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [ ] Update schema version `0.3` across source, checked-in JSON Schema, changelog, tests, README, and bundled skill references; repair the existing StatusResponse schema drift and document the global-version effect on completion/output-match event envelopes (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)
- [ ] Add integration tests named with the `status_` prefix. At minimum include `status_reports_process_alive_for_live_pid`, `status_reports_process_alive_false_for_dead_pid_fixture`, `status_omits_process_alive_for_terminal_job`, `status_terminal_reports_persisted_duration_ms`, `status_reports_signal_and_logs_drained`, `status_reports_log_paths_and_byte_totals`, `status_missing_logs_report_zero_bytes`, `status_large_logs_use_bounded_size_observation`, `status_omits_env_values_and_stdin_content`, `status_does_not_rewrite_state_json`, and `status_response_validates_against_checked_in_schema`; zero matched tests do not constitute evidence (verification: integration - `cargo test --test integration status_`; verification-id: status-contract-tests)

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate enrich-status-diagnostics --archive-gate`.

Conflux acceptance must also run `make check`; it is not delegated to `prek.toml` because those hooks are staged-file scoped.
