## MODIFIED Requirements

### Requirement: POST /exec によるジョブ起動

`POST /exec` MUST accept the existing required `command` and optional launch/observation fields, except that the seconds-based destructive workload lifetime limit MUST be named `max_runtime`. The request fields `timeout` and `timeout_ms` MUST be rejected before creating a job. `until` MUST continue to bound inline observation only and MUST NOT signal the managed job when it expires.

#### Scenario: POST /exec accepts max_runtime

**Given**: `agent-exec serve` is running
**When**: `/exec` receives a command that exceeds `max_runtime=1`
**Then**: the request creates the managed job successfully
**And**: the supervisor terminates the workload at the configured lifetime ceiling

#### Scenario: POST /exec rejects timeout

**Given**: `agent-exec serve` is running
**When**: `/exec` receives `{"command":["sleep","60"],"timeout":1}`
**Then**: the response is HTTP 400
**And**: no job is created

#### Scenario: POST /exec until expiry leaves the job running

**Given**: `/exec` receives a workload that runs longer than `until=1`
**When**: inline observation ends
**Then**: the response reports a non-terminal job
**And**: the service has not signaled the workload
