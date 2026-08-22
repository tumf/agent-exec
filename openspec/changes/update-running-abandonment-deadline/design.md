# Design: mutable running-job abandonment control

## Control record

The dependent rename change provides the launch-time `abandon-job-after` contract. This change adds a revisioned job-local control record as the runtime source of truth:

- `revision`: monotonically increasing integer
- `phase`: `active`, `disabled`, or `triggered`
- `abandon_job_after_ms`: duration accepted for the current revision, null when disabled
- `deadline`: absolute RFC 3339 timestamp, null when disabled
- `configured_by`: `launch` or `update`, null only for an unlimited job never configured destructively
- `updated_at`: acceptance timestamp

Launch materializes revision 1. Legacy jobs are materialized from the dependency's reconciled persisted metadata when start/restart or the first update needs mutable control. Read-only status may derive compatibility output but must not write.

## Public operations

- CLI set: `agent-exec abandon set <job_id> --in <seconds> --acknowledge-result-loss`
- CLI clear: `agent-exec abandon clear <job_id>`
- MCP: `set_abandonment(job_id, abandon_in, acknowledge_result_loss)` and `clear_abandonment(job_id)`
- HTTP: `PUT /jobs/{job_id}/abandonment` and `DELETE /jobs/{job_id}/abandonment`
- Public Rust/embedded methods with equivalent typed requests and responses

`--in` / `abandon_in` is relative to durable acceptance time, not original job start time. Every set requires acknowledgement because the resulting action remains destructive. Clear does not.

## Linearization and crash behavior

Updater and supervisor use the same job-local exclusive lock.

1. Set/clear acquires the lock, reads state and control, rejects non-running or triggered jobs, computes the full replacement, atomically writes it, and releases the lock. The successful atomic replacement is the update linearization point.
2. Before signaling, the supervisor acquires the lock and re-reads the control. It signals only when the current active revision is due. It first atomically changes that revision to `triggered`, then releases the lock and signals. Persisting `triggered` is the trigger linearization point.
3. If update commits first, a supervisor awakened for an older deadline observes the new revision and does not signal.
4. If trigger commits first, later set/clear returns `invalid_state` and never claims success.
5. A crash before atomic replacement leaves the old complete record. A crash after replacement leaves the new complete record. No partial JSON is admitted.
6. Multiple updaters serialize under the lock; each successful response returns its revision and deadline.

The supervisor observes control changes without busy waiting. It may check the small control record on a bounded cadence integrated into the current watcher. Status remains read-only.

## Time model

`deadline` is persisted wall-clock time so restart can preserve the accepted deadline. While one supervisor process remains alive, it should derive each wait interval from the latest wall deadline and recheck both wall clock and revision on wake. It must never signal solely because an earlier monotonic timer expired without re-reading the locked control.

Wall-clock movement can change real elapsed time to a persisted wall deadline. The contract is explicit:

- Forward movement can make a deadline immediately due.
- Backward movement delays it until wall time reaches the persisted deadline.
- `abandon_remaining_ms` is therefore advisory and response-time based, not a scheduler guarantee.

This avoids inventing a restart-stable monotonic clock. Tests use injected/fake clock boundaries where needed rather than changing the system clock.

## Status projection

`status` reads the effective control and returns:

- `abandon_job_after_ms`: current revision duration; null when disabled
- `abandon_deadline`: current absolute deadline; null when disabled
- `abandon_remaining_ms`: `max(deadline - response_time, 0)` for a running active job; null when disabled or terminal
- `abandon_revision`: current durable revision
- `abandon_configured_by`: `launch` or `update`; null for never-configured unlimited jobs

These fields are diagnostic. The supervisor's locked control transition, not a status-derived remaining value, decides firing.

## Restart

Restart reads the latest complete control record and revision. It does not restore launch metadata over an updated record. If the persisted active deadline is already due, the new supervisor uses the same locked active-to-triggered transition before signaling the restarted workload.

## Verification strategy

Use `mutable_abandonment`-prefixed tests across CLI, MCP, and HTTP targets, executed by `cargo test mutable_abandonment`:

- set on an unlimited job;
- shorten and extend an active deadline;
- clear before the old deadline and prove the workload survives;
- reject missing/false acknowledgement without mutation;
- reject created, terminal, and triggered states;
- deterministic lock-controlled update-wins and trigger-wins races;
- concurrent updaters return distinct ordered revisions;
- crash-safe atomic replacement fixtures;
- restart from an updated future deadline and an already-due deadline;
- status active/disabled/terminal/null semantics and remaining-time clamping;
- parity for CLI, MCP, HTTP, embedded, and public Rust operations.
