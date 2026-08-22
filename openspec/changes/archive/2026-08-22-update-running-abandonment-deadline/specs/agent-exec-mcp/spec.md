## ADDED Requirements

### Requirement: MCP exposes mutable abandonment control

MCP MUST expose `set_abandonment(job_id, abandon_in, acknowledge_result_loss)` and `clear_abandonment(job_id)`. Set MUST require true acknowledgement and interpret `abandon_in` relative to durable update acceptance. Both tools MUST use the canonical revisioned, fixed-lock-file update/trigger transition, reject running jobs without a compatible supervisor-authored control record and stable job-domain errors. MCP `status` MUST expose the canonical effective abandonment status fields.

#### Scenario: MCP replaces and reports a running deadline

**Given**: a running managed job exists
**When**: the client calls `set_abandonment` with `abandon_in=30` and true acknowledgement
**Then**: the response includes the new revision and absolute deadline
**And**: MCP status reports the same effective revision and deadline

#### Scenario: MCP clears a running deadline

**Given**: a running managed job has an active abandonment deadline
**When**: the client calls `clear_abandonment`
**Then**: the returned revision is disabled
**And**: the cleared deadline does not signal the job
