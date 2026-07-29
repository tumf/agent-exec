## ADDED Requirements

### Requirement: Unix detached supervisor child reaping

On Unix-like platforms, any long-lived agent-exec process that launches detached supervisors MUST retain ownership of each supervisor child until its exit status is collected. Supervisor reaping MUST NOT block the launch response, cancel the managed job, alter persisted job state, or wait indiscriminately for child processes not spawned by that launch path. All execution surfaces using the canonical supervisor launcher, including `run`, `start`, `restart`, HTTP serve, and MCP, MUST receive this behavior without surface-specific cleanup logic.

#### Scenario: long-lived MCP server reaps finished supervisors

**Given**: one `agent-exec mcp` server remains alive while it launches multiple short managed jobs
**When**: those jobs reach terminal state and their supervisor processes exit
**Then**: the MCP server collects each supervisor exit status
**And**: no exited supervisor remains as a zombie child of the MCP server
**And**: the MCP server remains responsive to subsequent protocol requests

#### Scenario: reaping preserves detached managed jobs

**Given**: a launcher starts a managed job whose supervisor remains active beyond the initial observation deadline
**When**: the launch response returns or an MCP client disconnects
**Then**: the supervisor and workload continue independently
**And**: the reaping mechanism does not signal or synchronously wait for job completion
**And**: later `status` or `wait` observes the canonical persisted job state

#### Scenario: reaping is scoped to owned supervisor children

**Given**: a long-lived agent-exec process may contain child processes created by different internal components or dependencies
**When**: a managed-job supervisor exits
**Then**: agent-exec waits for the specific supervisor `Child` it spawned
**And**: it does not install behavior that indiscriminately consumes another component's child exit status

#### Scenario: Windows process lifecycle remains unchanged

**Given**: agent-exec launches a supervisor on Windows
**When**: the supervisor is initialized and assigned through the existing Job Object handshake
**Then**: the Windows launch and handshake behavior remains unchanged
**And**: Unix-specific reaping behavior does not alter the Windows process contract
