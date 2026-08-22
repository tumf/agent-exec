## ADDED Requirements

### Requirement: HTTP exposes mutable abandonment control

The service MUST expose flat routes `PUT /abandon/{job_id}` with `abandon_in` and true acknowledgement, and `DELETE /abandon/{job_id}` for clearing, consistent with existing job-operation route naming. CORS configuration MUST permit PUT and DELETE. Both routes MUST use the canonical revisioned, fixed-lock-file update/trigger transition and reject running jobs without a compatible supervisor-authored control record and stable job-domain errors. Job status responses MUST expose the canonical effective abandonment status fields.

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

#### Scenario: CORS preflight admits abandonment updates

**Given**: serve runs with CORS enabled
**When**: a client sends an OPTIONS preflight for PUT or DELETE on `/abandon/{job_id}`
**Then**: the requested method is permitted
