## ADDED Requirements

### Requirement: running-job abandonment control is mutable and race-safe

A running job's active abandonment deadline MUST be replaceable through `agent-exec abandon set <job_id> --in <seconds> --acknowledge-result-loss` and removable through `agent-exec abandon clear <job_id>`. Set MUST interpret `--in` relative to durable update acceptance and MUST require true result-loss acknowledgement. Clear MUST be non-destructive and MUST NOT require acknowledgement. Created, terminal, or already-triggered jobs MUST reject changes with a stable job-domain invalid-state error.

Updater and supervisor MUST serialize set, clear, and trigger through advisory locking on the same fixed-name per-job lock file and a revisioned atomically replaced control record. A compatible supervisor MUST author the control record and run its observer for every managed workload, including unlimited jobs. A running job without a supervisor-authored control record MUST reject set/clear with stable `invalid_state` and restart guidance; it MUST NOT report metadata-only success. The supervisor MUST re-read the current locked revision before signaling and MUST atomically persist the current revision as triggered before signaling. If update commits first, the stale deadline MUST NOT signal the job. If trigger commits first, the update MUST fail without claiming success. Restart MUST rearm `configured_by="launch"` controls from their configured duration, preserve the accepted absolute deadline and revision for `configured_by="update"`, and resume signal/escalation for `triggered` control when the workload remains alive. Accepted set/clear MUST synchronize both compatibility metadata duration fields; clear MUST write zero to both.

#### Scenario: an update wins before the old deadline triggers

**Given**: a running job has an active abandonment deadline
**When**: `abandon set` atomically commits a later deadline and new revision before the supervisor marks the old revision triggered
**Then**: the supervisor observes the new revision
**And**: the old deadline does not signal the workload

#### Scenario: a trigger wins before an update

**Given**: an update and a due deadline contend for the job-local lock
**When**: the supervisor atomically marks the current revision triggered first
**Then**: the later update fails with `invalid_state`
**And**: it does not claim to have prevented abandonment

#### Scenario: clearing preserves the running workload

**Given**: a running job has an active deadline
**When**: `abandon clear` commits a disabled revision before the deadline triggers
**Then**: the prior deadline never signals the workload
**And**: the job remains managed and observable

#### Scenario: restart preserves the latest updated deadline

**Given**: a running job has a durably updated future deadline and revision
**When**: the job is restarted
**Then**: the new supervisor resumes that effective deadline and revision
**And**: it does not restore the launch-time deadline

### Requirement: status exposes effective abandonment control

`status` MUST report `abandon_job_after_ms`, `abandon_deadline`, `abandon_remaining_ms`, `abandon_revision`, and `abandon_configured_by` from the effective durable control. Remaining time MUST be computed at response time, clamp to `[0, abandon_job_after_ms]`, and be omitted/null when the control is disabled, the job is terminal, or the control record is absent. Revision and configuration source MUST be null when the control record is absent. Remaining time is advisory and MUST NOT be used as the supervisor's firing source of truth. Status MUST remain read-only.

#### Scenario: status reports an updated active deadline

**Given**: a running job's abandonment deadline was updated
**When**: status is queried
**Then**: duration, absolute deadline, revision, and `configured_by="update"` match the durable control
**And**: remaining milliseconds are clamped to the inclusive range from zero through the configured duration

#### Scenario: status reports a disabled deadline

**Given**: a running job's abandonment deadline was cleared
**When**: status is queried
**Then**: duration, deadline, and remaining fields are null
**And**: the durable revision identifies the clear operation

#### Scenario: a legacy running supervisor cannot accept a false update

**Given**: a running job has no supervisor-authored abandonment control record
**When**: set or clear is requested
**Then**: the operation fails with `invalid_state` and restart guidance
**And**: neither control state nor compatibility metadata is mutated

#### Scenario: an unlimited job observes a newly set deadline

**Given**: a compatible unlimited job runs without progress reporting
**When**: set accepts a deadline
**Then**: the unconditional control observer triggers abandonment at the effective deadline

#### Scenario: restart recovers a triggered transition

**Given**: the control revision is persisted as triggered but the workload is still alive
**When**: a compatible supervisor restarts or recovers the job
**Then**: signal and escalation resume from the persisted transition

#### Scenario: clear remains safe across downgrade metadata reading

**Given**: clear was accepted during the dual-write compatibility window
**When**: an older reader loads the compatibility metadata
**Then**: both duration fields are zero
**And**: the cleared launch deadline is not rearmed

#### Scenario: status reports no mutable control for a legacy job

**Given**: a job has no supervisor-authored control record
**When**: status is queried
**Then**: abandonment revision, configuration source, deadline, and remaining fields are null
**And**: status performs no materialization write
