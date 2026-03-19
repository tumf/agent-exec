## MODIFIED Requirements

### Requirement: run の監視分離

`run` must keep job completion detection tied to the wrapped root process, not to the eventual closure of inherited stdout/stderr handles held by descendants. Once the wrapped root process exits, the supervisor must persist a terminal job state promptly, and `status` / `wait` must stop reporting `running` even if descendant processes still hold the inherited log pipes open.

#### Scenario: parent exits while descendant still holds inherited stdio

Given `agent-exec run --snapshot-after 0 -- sh -c '<parent exits successfully after spawning a descendant that keeps inherited stdout/stderr open briefly>'` is executed
When the wrapped root process exits successfully
Then `agent-exec status <job_id>` eventually reports `exited` without remaining `running` indefinitely
And `agent-exec wait <job_id>` returns a terminal state without depending on descendant pipe closure
And `_supervise` exits promptly after persisting the terminal state
