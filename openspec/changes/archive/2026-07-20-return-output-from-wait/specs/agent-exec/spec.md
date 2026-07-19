## ADDED Requirements

### Requirement: wait returns bounded job output

The `wait` command MUST return the job output available when the wait observation ends. The response MUST use bounded `stdout` and `stderr` excerpts with the existing range, total-byte, and encoding metadata contract. Complete output MUST remain persisted in the job logs. CLI, HTTP `GET /wait/:id`, and MCP `wait` MUST expose equivalent behavior through the shared wait response path.

#### Scenario: terminal wait returns final output

**Given**: A managed job writes to stdout and stderr and then exits
**When**: The caller waits until the job reaches a terminal state
**Then**: The successful wait response includes the final bounded stdout and stderr excerpts
**And**: The response includes range, total-byte, and encoding metadata
**And**: The response includes the terminal state and exit code

#### Scenario: deadline wait returns current output

**Given**: A managed job writes output and remains running beyond the wait observation deadline
**When**: The wait deadline expires
**Then**: The successful wait response includes the bounded output available at that time
**And**: The response reports a non-terminal state without inventing an exit code

#### Scenario: large output remains bounded

**Given**: A managed job produces output larger than the inline-output ceiling
**When**: The caller waits for completion
**Then**: The wait response contains only a bounded excerpt and accurate range and total-byte metadata
**And**: The complete output remains available in the persisted logs and through `tail`
