## Implementation Tasks

- [x] Reconcile effective list state for persisted `running` jobs before state filtering. Completion condition: `src/list.rs` or equivalent list construction code maps `running` jobs with dead or missing persisted PIDs to `unknown` before `JobSummary` creation and before `--state` filtering. (verification: integration - `cargo test --test integration ps_excludes_stale_running_job_with_dead_pid`)

- [x] Add cross-platform process liveness helpers for list-time reconciliation. Completion condition: Unix/macOS checks `kill(pid, 0)` and treats `EPERM` as alive; Windows checks `OpenProcess`/`GetExitCodeProcess` for `STILL_ACTIVE`; unsupported platforms avoid false dead classification. (verification: unit - inspect `src/list.rs` helper branches and run `cargo clippy --all-targets --all-features -- -D warnings`; integration behavior is covered by `cargo test --test integration ps_excludes_stale_running_job_with_dead_pid`.)

- [x] Add a regression integration test for stale `running` jobs. Completion condition: `tests/integration.rs` creates a fake job with persisted `running` state and a non-existent PID, asserts `ps --all` excludes it, and asserts `list --all` reports it as `unknown`. (verification: integration - `cargo test --test integration ps_excludes_stale_running_job_with_dead_pid`)

- [x] Verify live `running` and terminal jobs still behave correctly. Completion condition: existing `list_filters_by_state_running`, `ps_returns_only_running_jobs`, and `ps_all_includes_running_jobs_from_other_cwds` tests continue to pass. (verification: integration - `cargo test --test integration list_filters_by_state_running ps_returns_only_running_jobs ps_all_includes_running_jobs_from_other_cwds`)

- [x] Run full repository quality gates. Completion condition: formatting, linting, and tests pass without JSON contract regressions. (verification: integration - `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features -- -D warnings`; `cargo test --all`)

## Future Work

- Add a separate proposal if operators need a public `stale` or `orphaned` state in the JSON schema.
- Add supervisor PID persistence or process-table supervisor discovery if child PID liveness is insufficient for future stale detection requirements.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate fix-stale-running-ps --archive-gate`
