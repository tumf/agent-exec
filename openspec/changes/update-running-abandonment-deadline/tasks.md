## Implementation Tasks

- [ ] Add the revisioned atomic job-local abandonment control record, migration/materialization from the dependency's persisted fields, and shared exclusive locking for updater/supervisor transitions (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Refactor supervisor timing to re-read the current locked revision before signaling, observe updates on a bounded cadence, persist `triggered` before signaling, and resume the latest absolute deadline after restart (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Add CLI `abandon set <job_id> --in <seconds> --acknowledge-result-loss` and `abandon clear <job_id>` with running/active admission, stable invalid-state errors, and revision/deadline responses (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Add equivalent MCP set/clear tools, HTTP PUT/DELETE abandonment routes, and public Rust/embedded operations using the canonical locked control transition and error contract (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Extend status responses and published schemas with effective duration, absolute deadline, response-time remaining milliseconds, revision, and configuration source; preserve read-only behavior and documented null/clamp semantics (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Add `mutable_abandonment`-prefixed real-job, deterministic race, concurrent updater, crash fixture, restart, clock-boundary, status, CLI, MCP, and HTTP tests that fail for stale timers or metadata-only no-op implementations (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)
- [ ] Update README, CHANGELOG, site/docs, bundled skill references, schemas, and examples with runtime set/clear operations, status fields, race semantics, and the advisory nature of remaining time (verification: integration - `cargo test mutable_abandonment`; verification-id: mutable-abandonment-tests)

## Notes

- This change remains blocked until `rename-timeout-to-abandon-job-after` is archived and integrated because it consumes that change's public names, acknowledgement contract, persisted metadata, and result markers.
- `abandon_remaining_ms` is diagnostic. The supervisor decides firing only through the locked control record.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate update-running-abandonment-deadline --archive-gate`.

Conflux acceptance must also run `cargo test mutable_abandonment`, `make check`, and `git diff --check`.
