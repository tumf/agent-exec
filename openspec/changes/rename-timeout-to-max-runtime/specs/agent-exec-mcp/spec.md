## MODIFIED Requirements

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` MUST accept a required non-empty `command` string array and the existing optional launch and observation controls, except that the destructive workload lifetime limit MUST be named `max_runtime` rather than `timeout`. `max_runtime` is seconds-based and MUST map to the canonical managed-job lifetime ceiling. The legacy `timeout` and `timeout_ms` request fields MUST be rejected before workload launch. `until` MUST remain a bounded observation duration whose expiry never stops the detached job.

#### Scenario: MCP max_runtime terminates the workload

**Given**: the client calls MCP `run` with a command that runs longer than `max_runtime=1`
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
