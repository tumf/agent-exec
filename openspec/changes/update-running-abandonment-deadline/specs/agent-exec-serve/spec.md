## ADDED Requirements

### Requirement: HTTP exposes mutable abandonment control

The service MUST expose `PUT /jobs/{job_id}/abandonment` with `abandon_in` and true acknowledgement, and `DELETE /jobs/{job_id}/abandonment` for clearing. Both routes MUST use the canonical revisioned, locked update/trigger transition and stable job-domain errors. Job status responses MUST expose the canonical effective abandonment status fields.

#### Scenario: HTTP replaces and reports a running deadline

**Given**: a running managed job exists
**When**: PUT abandonment accepts `abandon_in=30` with true acknowledgement
**Then**: the response includes the new revision and absolute deadline
**And**: subsequent status reports the same effective control

#### Scenario: HTTP clears a running deadline

**Given**: a running managed job has an active abandonment deadline
**When**: DELETE abandonment succeeds
**Then**: the durable control becomes disabled at a new revision
**And**: the cleared deadline does not signal the job
