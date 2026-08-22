## Implementation Tasks

- [ ] Rename CLI launch controls from `--timeout` to `--abandon-job-after` across `run`, `create`, hidden supervisor handoff, restart/start plumbing, help, and shell completions; begin help with the unfinished-result-loss warning, require `--acknowledge-result-loss`, and reject old spelling or missing acknowledgement without creating a job (verification: integration - `cargo test --test integration abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Rename MCP `run.timeout` and HTTP `/exec` `timeout` to `abandon_job_after`, put the result-loss warning in field descriptions, require `acknowledge_result_loss=true`, reject old/dual/unacknowledged requests before launch, and keep `until` strictly non-destructive (verification: integration - `cargo test --test integration abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Rename the embedded Rust launch request field to `abandon_job_after_ms`, require the explicit result-loss acknowledgement boolean when nonzero, and update public schema/types and warning docs while preserving current signal escalation and terminal-state serialization (verification: integration - `cargo test --test integration abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Migrate persisted metadata writes to `abandon_job_after_ms`; accept legacy stored `timeout_ms` only when reading existing job definitions, reject conflicting dual fields, and prove legacy start/restart behavior (verification: integration - `cargo test --test integration abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Update canonical specs, README, changelog, site/docs, bundled skills, examples, and test fixtures so current guidance uses `abandon-job-after` / `abandon_job_after`, distinguishes it from `until`, and contains no live public `timeout` launch examples (verification: integration - `cargo test --test integration abandon_job_after`; verification-id: abandon-job-contract-tests)
- [ ] Add `abandon_job_after`-prefixed integration coverage for warning text, missing/false acknowledgement rejection, acknowledged real workload termination, default unlimited runtime, non-destructive `until`, old public spelling rejection on every launch surface, new metadata output, legacy metadata read/restart, and conflicting metadata rejection (verification: integration - `cargo test --test integration abandon_job_after`; verification-id: abandon-job-contract-tests)

## Notes

- Preserve terminal `state="timeout"` for result compatibility; only the control input is renamed.
- Legacy compatibility is confined to persisted metadata reads. Public invocation aliases are intentionally excluded because they preserve the ambiguity that caused workload loss.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate rename-timeout-to-abandon-job-after --archive-gate`.

Conflux acceptance must also run `make check`; staged-file hooks are not an unconditional clean-tree gate.
