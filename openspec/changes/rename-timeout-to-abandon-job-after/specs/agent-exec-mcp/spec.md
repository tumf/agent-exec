## MODIFIED Requirements

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` MUST accept a required non-empty `command` string array and the existing optional launch and observation controls, except that the destructive workload lifetime limit MUST be named `abandon_job_after` rather than `timeout`. `abandon_job_after` is seconds-based and MUST carry a warning that it gives up on the job and may permanently lose unfinished results. A non-null `abandon_job_after` MUST require `acknowledge_result_loss=true`; missing or false acknowledgement MUST fail before persistence or launch. It MUST map to the canonical managed-job lifetime ceiling. The legacy `timeout` and `timeout_ms` request fields MUST be rejected before workload launch. `until` MUST remain a bounded observation duration whose expiry never stops the detached job.

#### Scenario: MCP abandon_job_after terminates the workload

**Given**: the client calls MCP `run` with a command that runs longer than `abandon_job_after=1` and `acknowledge_result_loss=true`
**When**: the managed lifetime ceiling is reached
**Then**: the supervisor terminates the workload using the canonical escalation behavior

#### Scenario: MCP rejects timeout before launch

**Given**: the client calls MCP `run` with `timeout=1`
**When**: request input is validated
**Then**: the tool returns a protocol-safe invalid-input error
**And**: no job is created

#### Scenario: MCP until remains non-destructive

**Given**: the client starts a workload that runs longer than `until=1`
**When**: the observation deadline expires
**Then**: MCP returns a canonical non-terminal run envelope
**And**: a subsequent status call observes the same running job

#### Scenario: MCP rejects unacknowledged abandonment

**Given**: the client calls MCP `run` with `abandon_job_after=1` and no true acknowledgement
**When**: request input is validated
**Then**: the tool returns a protocol-safe error containing the result-loss warning
**And**: no job is created
