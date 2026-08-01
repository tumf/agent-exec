## MODIFIED Requirements

### Requirement: run の監視分離

Issue `#5` verification must distinguish between visible success output and actual workload termination. A job must not be considered reliably complete merely because its logs contain apparent success lines, and regressions for lingering `running` state must include a reproduction shape where the wrapped workload process itself may remain alive after success-like output (MUST). The same detached supervisor and workload-liveness semantics SHALL apply whether a job is launched through the standalone CLI, MCP/HTTP adapters, or the embedded Rust API. Embedded launch SHALL re-execute an explicitly selected supervisor executable and SHALL NOT substitute an in-process thread whose lifetime is tied to the caller.

#### Scenario: cflx-like workload logs success before job leaves running

Given a workload launched via `agent-exec run -- <workload>` emits success-like completion lines to stdout
And the job still has a live wrapped workload process after those lines are visible
When `agent-exec status <job_id>` and `agent-exec wait <job_id>` are evaluated for issue `#5`
Then the regression analysis must treat this as a distinct failure shape from descendant-held stdio only
And any accepted fix must be verified against this workload-liveness case, not only shell-only synthetic cases

#### Scenario: Embedded launcher exits while workload continues

**Given**: a consumer links the agent-exec crate, delegates the reserved supervisor invocation at startup, and launches a managed job through the typed API
**When**: the original consumer process exits before the workload
**Then**: the detached supervisor continues recording output and terminal state and another client instance can observe and control the same job through the explicit jobs root

#### Scenario: Embedded supervisor delegation is missing

**Given**: a consumer selects its own executable for supervision but does not delegate the reserved startup invocation
**When**: the typed API attempts to launch a job
**Then**: launch fails without reporting success or leaving durable state that falsely identifies a nonexistent supervisor as running
