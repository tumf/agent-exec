# Design: mutable running-job abandonment control

## Control record

The dependent rename change provides the launch-time `abandon-job-after` contract. This change adds a revisioned job-local control record as the runtime source of truth:

- `revision`: monotonically increasing integer
- `phase`: `active`, `disabled`, or `triggered`
- `abandon_job_after_ms`: duration accepted for the current revision, null when disabled
- `deadline`: absolute RFC 3339 timestamp, null when disabled
- `configured_by`: `launch` or `update`, null only for an unlimited job never configured destructively
- `updated_at`: acceptance timestamp

A compatible supervisor authors revision 1 before it can enforce abandonment. A running job without a supervisor-authored control record is treated as controlled by an incompatible legacy supervisor: set/clear returns stable `invalid_state` with restart guidance and does not materialize or mutate metadata. Start/restart may materialize legacy persisted metadata because the newly launched supervisor is compatible. Read-only status may derive compatibility output but must not write; record-absent status exposes null revision/source fields.

## Public operations

- CLI set: `agent-exec abandon set <job_id> --in <seconds> --acknowledge-result-loss`
- CLI clear: `agent-exec abandon clear <job_id>`
- MCP: `set_abandonment(job_id, abandon_in, acknowledge_result_loss)` and `clear_abandonment(job_id)`
- HTTP: `PUT /abandon/{job_id}` and `DELETE /abandon/{job_id}` using the existing flat route namespace; CORS permits PUT and DELETE
- Public Rust/embedded methods with equivalent typed requests and responses

`--in` / `abandon_in` is relative to durable acceptance time, not original job start time. Every set requires acknowledgement because the resulting action remains destructive. Clear does not.

## Linearization and crash behavior

Updater and supervisor use advisory locking on the same fixed-name per-job lock file. The lock is never placed on the control JSON inode because atomic rename would replace that inode; process crash releases the advisory lock automatically.

1. Set/clear acquires the lock, reads state and control, rejects non-running or triggered jobs, computes the full replacement, atomically writes it, and releases the lock. The successful atomic replacement is the update linearization point.
2. Before signaling, the supervisor acquires the lock and re-reads the control. It signals only when the current active revision is due. It first atomically changes that revision to `triggered`, then releases the lock and signals. Persisting `triggered` is the trigger linearization point.
3. If update commits first, a supervisor awakened for an older deadline observes the new revision and does not signal.
4. If trigger commits first, later set/clear returns `invalid_state` and never claims success.
5. A crash before atomic replacement leaves the old complete record. A crash after replacement leaves the new complete record. No partial JSON is admitted.
6. Multiple updaters serialize under the lock; each successful response returns its revision and deadline.
7. After every accepted set/clear, the updater synchronizes the dependency's dual-written metadata duration fields. Set writes the new duration to both `abandon_job_after_ms` and legacy `timeout_ms`; clear writes zero to both. A downgrade may lose an updated absolute deadline but must not resurrect a cleared launch limit.
8. If a process crashes after persisting `triggered` but before signaling, restart or a replacement compatible supervisor detects `triggered` with a live workload and resumes signal/escalation instead of leaving it permanently unmanageable.

The compatible supervisor runs the control observer for every managed workload, including unlimited jobs with progress reporting disabled. It checks the small control record on a bounded cadence integrated into the current watcher and never reports set success when no compatible observer exists. Status remains read-only.

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
- `abandon_remaining_ms`: `clamp(deadline - response_time, 0, abandon_job_after_ms)` for a running active job; null when disabled, terminal, or the control record is absent
- `abandon_revision`: current durable revision; null when the control record is absent
- `abandon_configured_by`: `launch` or `update`; null for never-configured unlimited jobs

These fields are diagnostic. The supervisor's locked control transition, not a status-derived remaining value, decides firing.

## Restart

Restart reads the latest complete control record and revision:

- `configured_by="launch"`: rearm from the configured duration at restart, preserving the existing restart contract.
- `configured_by="update"`: retain the accepted absolute deadline and revision; an already-due deadline uses the locked active-to-triggered transition immediately.
- `phase="triggered"` with a live workload: resume signal and escalation from the persisted transition.

Restart never restores launch metadata over an update-configured record.

## Verification strategy

Use `mutable_abandonment`-prefixed tests across CLI, MCP, and HTTP targets, executed by `cargo test mutable_abandonment`:

- reject set/clear on a legacy running job with no supervisor-authored control record;
- set on an unlimited job with no progress watcher and prove the unconditional observer fires;
- shorten and extend an active deadline;
- clear before the old deadline and prove the workload survives;
- reject missing/false acknowledgement without mutation;
- reject created, terminal, and triggered states;
- deterministic lock-controlled update-wins and trigger-wins races;
- concurrent updaters return distinct ordered revisions;
- crash-safe atomic replacement fixtures;
- restart launch-configured duration rearming, update-configured absolute preservation, an already-due deadline, and triggered-before-signal recovery;
- dual-written metadata synchronization on set/clear and downgrade-reader behavior;
- status active/disabled/terminal/missing-record null semantics and `[0, duration]` remaining-time clamping;
- CORS PUT/DELETE preflight and flat HTTP route behavior;
- parity for CLI, MCP, HTTP, embedded, and public Rust operations.
