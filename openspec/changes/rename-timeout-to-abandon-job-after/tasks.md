## Implementation Tasks

- [ ] Rename public CLI launch controls from `--timeout` to `--abandon-job-after` across `run` and `create`; begin help with the unfinished-result-loss warning, require `--acknowledge-result-loss`, and keep hidden `--timeout` only as an always-failing migration trap that identifies both `--abandon-job-after` and non-destructive `--until` without creating a job (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Keep private `_supervise --timeout` wire syntax unchanged, but rename canonical internal/public Rust fields across `main.rs`, `run::RunOpts`, `run::SuperviseOpts`, `create::CreateOpts`, `start.rs`, and `restart.rs`; test `spawn_supervisor_process` against `embedded.rs::SuperviseArgs` so producer/parser drift cannot compile successfully and fail only at runtime (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Rename MCP `run.timeout` and HTTP `/exec` `timeout` to `abandon_job_after`, put the result-loss warning in field descriptions, require `acknowledge_result_loss=true`, return actionable migration errors for legacy fields before launch, and keep `until` strictly non-destructive (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Rename the embedded public Rust launch request field to `abandon_job_after_ms`, require the explicit result-loss acknowledgement boolean when nonzero, and update public schema/types/docs while preserving signal escalation and private supervisor serialization (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Implement explicit persisted metadata reconciliation: dual-write equal `abandon_job_after_ms` and `timeout_ms` for one migration release; accept legacy-only, new-only, and equal dual definitions; reject unequal dual fields with job ID and both values; prove start/restart behavior and old-binary downgrade preservation (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Add additive actual-abandonment markers `abandoned_by="abandon_job_after"` and `result_loss=true` to persisted state and its status/list/completion-event projections while preserving terminal `state="timeout"`; do not emit markers when the configured limit never fires or for historical state lacking provenance (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Update canonical specs, README, CHANGELOG, site/docs, bundled skills, schema output, shell completions, examples, and fixtures so current guidance uses `abandon-job-after` / `abandon_job_after`, explains legacy `state="timeout"`, distinguishes it from `until`, and contains no live public `timeout` launch syntax except migration/rejection text (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Add `abandon_job_after_`-prefixed coverage in `tests/integration.rs`, `tests/mcp_integration.rs`, and `tests/serve_integration.rs` for warning text, missing/false acknowledgement rejection, acknowledged real workload termination, default unlimited runtime, non-destructive `until`, actionable old-public-spelling rejection, abandonment markers, legacy/new/equal-dual/unequal-dual metadata, start/restart, downgrade compatibility, and public docs grep gates (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)

## Notes

- Preserve terminal `state="timeout"` for result compatibility; additive fields identify actual abandonment.
- Legacy public inputs never launch. Persisted compatibility is deliberately broader because existing and downgrade-operated jobs must retain their configured limit.
- Removing the dual-written legacy metadata field requires a later explicit migration change.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate rename-timeout-to-abandon-job-after --archive-gate`.

Conflux acceptance must also run `cargo test abandon_job_after`, `make check`, and `git diff --check`; staged-file hooks are not an unconditional clean-tree gate.
