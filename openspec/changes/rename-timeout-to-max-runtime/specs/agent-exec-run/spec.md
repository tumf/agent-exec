## MODIFIED Requirements

### Requirement: timeout と kill-after

Public launch surfaces MUST name the workload lifetime ceiling `max-runtime` in CLI form and `max_runtime` in structured request form. When configured, reaching `max-runtime` MUST send the termination signal, and a process still alive after `kill-after` MUST be force-killed. The default value MUST remain unlimited. Current public launch surfaces MUST reject the ambiguous legacy input name `timeout` before creating a job.

Persisted definitions written by the new implementation MUST use `max_runtime_ms`. Readers MUST continue to accept existing definitions containing only legacy `timeout_ms`, preserving start and restart behavior. A persisted definition containing both fields MUST fail closed. Existing terminal state serialization as `timeout` MUST remain compatible.

#### Scenario: max-runtime terminates the workload

**Given**: `agent-exec run --max-runtime 1 --kill-after 1 -- sleep 60` is executed
**When**: two seconds elapse
**Then**: the managed process tree is no longer running
**And**: the terminal result remains representable by the existing timeout state

#### Scenario: legacy public timeout spelling is rejected

**Given**: a user executes `agent-exec run --timeout 1 -- sleep 60`
**When**: CLI arguments are validated
**Then**: the command fails with a usage error
**And**: no job is created

#### Scenario: legacy persisted metadata remains restartable

**Given**: an existing job definition contains `timeout_ms` and does not contain `max_runtime_ms`
**When**: the job is started or restarted
**Then**: the stored workload lifetime limit is preserved
**And**: newly written metadata uses `max_runtime_ms`

#### Scenario: conflicting persisted runtime fields fail closed

**Given**: persisted metadata contains both `timeout_ms` and `max_runtime_ms`
**When**: the definition is loaded
**Then**: loading fails with a clear invalid-definition error

### Requirement: 人間向け runtime 制御時間は秒単位である

`run`, `create`, and `_supervise` human-facing runtime controls (`--max-runtime`, `--kill-after`, `--progress-every`) MUST be interpreted in seconds. Internal conversion to milliseconds is allowed, but help, README, skills, and integration tests MUST agree on the second-based contract. `--until` MUST remain the observation deadline name and its expiry MUST NOT signal the managed job.

#### Scenario: run max-runtime is interpreted in seconds

**Given**: `agent-exec run --max-runtime 30 -- sh -c "sleep 60"` is executed
**When**: the workload lifetime limit is applied
**Then**: `30` is interpreted as 30 seconds
**And**: it is not interpreted as 30 milliseconds

#### Scenario: until expiry preserves the workload

**Given**: a managed job runs longer than one second
**When**: an observation returns after `until=1`
**Then**: the response is non-terminal
**And**: the managed job receives no termination signal
