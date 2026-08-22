## Implementation Tasks

- [ ] Add the revisioned atomic job-local abandonment control record, migration/materialization from the dependency's persisted fields, and fixed-name per-job advisory locking for updater/supervisor transitions, rejecting running jobs without a supervisor-authored record (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Refactor supervisor timing to run the control observer unconditionally, re-read the current locked revision before signaling, observe updates on a bounded cadence, persist `triggered` before signaling, recover triggered-before-signal crashes, rearm launch-configured duration, and preserve update-configured absolute deadlines after restart (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Synchronize dependency-era dual-written metadata fields after every accepted set/clear so downgrade cannot resurrect a cleared deadline (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Add CLI `abandon set <job_id> --in <seconds> --acknowledge-result-loss` and `abandon clear <job_id>` with running/active admission, stable invalid-state errors, and revision/deadline responses (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Add equivalent MCP set/clear tools, flat HTTP `PUT /abandon/{job_id}` / `DELETE /abandon/{job_id}` routes with CORS PUT/DELETE support, and public Rust/embedded operations using the canonical locked control transition and error contract (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Extend status responses and published schemas with effective duration, absolute deadline, response-time remaining milliseconds, revision, and configuration source; preserve read-only behavior and documented missing-record/null and `[0, duration]` clamp semantics (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Add `mutable_abandonment`-prefixed real-job, deterministic race, legacy-supervisor rejection, unconditional observer, concurrent updater, fixed-lock crash fixture, triggered recovery, restart-mode, downgrade metadata, clock-boundary, status, CLI, MCP, CORS, and HTTP tests that fail for stale timers or metadata-only no-op implementations (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Update README, CHANGELOG, site/docs, bundled skill references, schemas, and examples with runtime set/clear operations, status fields, race semantics, and the advisory nature of remaining time (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)

## Notes

- This change remains blocked until `rename-timeout-to-abandon-job-after` is archived and integrated because it consumes that change's public names, acknowledgement contract, persisted metadata, and result markers.
- `abandon_remaining_ms` is diagnostic. The supervisor decides firing only through the locked control record.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate update-running-abandonment-deadline --archive-gate`.

Conflux acceptance must first prove a non-empty focused test listing with `cargo test mutable_abandonment -- --list | grep -q mutable_abandonment`, then run `cargo test mutable_abandonment`, `make check`, and `git diff --check`.
