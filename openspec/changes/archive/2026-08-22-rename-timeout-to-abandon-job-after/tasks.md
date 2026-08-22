## Implementation Tasks

- [x] Rename public CLI launch controls from `--timeout` to `--abandon-job-after` across `run` and `create`; begin help with the unfinished-result-loss warning, require `--acknowledge-result-loss`, and keep hidden `--timeout` only as an always-failing migration trap that identifies both `--abandon-job-after` and non-destructive `--until` without creating a job (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Keep private `_supervise --timeout` wire syntax unchanged, but rename canonical internal/public Rust fields across `main.rs`, `run::RunOpts`, `run::SuperviseOpts`, `create::CreateOpts`, `start.rs`, and `restart.rs`; test `spawn_supervisor_process` against `embedded.rs::SuperviseArgs` so producer/parser drift cannot compile successfully and fail only at runtime (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Rename MCP `run.timeout` and HTTP `/exec` `timeout` to `abandon_job_after`, put the result-loss warning in field descriptions, require `acknowledge_result_loss=true`, return actionable migration errors for legacy fields before launch, and keep `until` strictly non-destructive (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Rename the embedded public Rust launch request field to `abandon_job_after_ms`, require the explicit result-loss acknowledgement boolean when nonzero, and update public schema/types/docs while preserving signal escalation and private supervisor serialization (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Implement explicit persisted metadata reconciliation: dual-write equal `abandon_job_after_ms` and `timeout_ms` for one migration release; accept legacy-only, new-only, and equal dual definitions; reject unequal dual fields with job ID and both values; prove start/restart behavior and old-binary downgrade preservation (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Add additive actual-abandonment markers `abandoned_by="abandon_job_after"` and `result_loss=true` to persisted state and its status/list/completion-event projections while leaving the existing terminal state value unchanged; do not emit markers when the configured limit never fires or for historical state lacking provenance (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Update spec deltas, README, CHANGELOG, site/docs, bundled skills, schema output, shell completions, examples, and fixtures so current guidance uses `abandon-job-after` / `abandon_job_after`, explains the additive abandonment markers, distinguishes the control from `until`, and contains no live public `timeout` launch syntax except migration/rejection text (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)
- [x] Add `abandon_job_after_`-prefixed coverage in `tests/integration.rs`, `tests/mcp_integration.rs`, and `tests/serve_integration.rs` for warning text, missing/false acknowledgement rejection, acknowledged real workload termination, default unlimited runtime, non-destructive `until`, actionable old-public-spelling rejection, abandonment markers, legacy/new/equal-dual/unequal-dual metadata, start/restart, downgrade compatibility, and public docs grep gates (verification: integration - `cargo test abandon_job_after`; verification-id: abandon-job-contract-tests)

## Notes

- Deviation from the original premise, recorded for acceptance: this codebase has **no terminal state value `timeout`**. `JobStatus` is `created | running | exited | killed | failed`, and a workload terminated by the runtime limit already surfaces as `killed` with its terminating signal. The proposal's "preserve terminal `state="timeout"`" is therefore implemented as "do not change the terminal state value at all", and the additive `abandoned_by` / `result_loss` pair carries the provenance the proposal asked for. `proposal.md`, `design.md`, and the `agent-exec-run` spec delta were corrected so the archived spec does not assert a state value that does not exist.
- Legacy public inputs never launch. Persisted compatibility is deliberately broader because existing and downgrade-operated jobs must retain their configured limit.
- Removing the dual-written legacy metadata field requires a later explicit migration change.
- Acknowledgement is enforced twice on purpose: at the CLI boundary as a clap usage error (exit 2, message naming CLI flags) and again inside `run::run_response` / `create::execute` so MCP, HTTP, and embedded Rust callers cannot bypass it.
- Reader reconciliation runs at `start` / `restart`, not inside deserialization, so a definition with divergent fields still lists, statuses, and deletes normally instead of becoming unreadable.
- An `agent-exec` capability delta was added for the `run`/`create`/`_supervise` seconds requirement, which named the removed public `--timeout` spelling and was not covered by the original three deltas.
- `tests/serve_integration.rs` gained a port-race repair: `free_port` closes its listener before `serve` binds it, so concurrently starting servers could be handed the same port and one would exit with "address already in use". The harness now waits for `/health` from the server it spawned and retries on a fresh port. Without it, `test_exec_empty_body_returns_400` failed intermittently once the four new `/exec` tests raised concurrency.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate rename-timeout-to-abandon-job-after --archive-gate`.

Conflux acceptance must also run `cargo test abandon_job_after`, `make check`, and `git diff --check`; staged-file hooks are not an unconditional clean-tree gate.

Results observed during apply:

- `cflx openspec validate rename-timeout-to-abandon-job-after --strict` passed.
- `cflx openspec validate rename-timeout-to-abandon-job-after --archive-gate` passed.
- `cargo test abandon_job_after` passed (29 tests: 7 unit, 14 CLI integration, 4 MCP, 4 HTTP).
- `make check` (fmt-check, clippy `-D warnings`, `cargo test --all`) passed.
- `git diff --check` reported no whitespace errors.
