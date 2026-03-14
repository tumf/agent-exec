## MODIFIED Requirements

### Requirement: run completion notification configuration

`run` must support completion notification sinks (MUST). `--notify-command <command>` and `--notify-file <path>` must be accepted (MUST). `--notify-command` must be interpreted as a single shell command string rather than a JSON argv array (MUST). Notification configuration must be persisted in job metadata (MUST).

#### Scenario: command sink configuration is persisted

Given `agent-exec run --notify-command 'cat >/tmp/event.json' -- echo hi` is executed
When job metadata is created
Then `meta.json` stores the configured command sink as a string value

### Requirement: command sink and file sink delivery contract

The `--notify-command` sink must execute through the platform shell and receive the completion event JSON on stdin (MUST). `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and `AGENT_EXEC_EVENT_TYPE` must be added to the sink environment (MUST). The `--notify-file` sink must append one completion event JSON line as NDJSON and create parent directories when needed (MUST).

#### Scenario: command sink receives event JSON and env variables

Given `agent-exec run --notify-command 'cat > /tmp/event.json' -- echo done` is executed on a Unix-like platform
When the job finishes
Then the shell command receives completion event JSON on stdin
And `AGENT_EXEC_EVENT_TYPE=job.finished` is present in the environment

#### Scenario: command sink uses platform shell execution on Windows

Given `agent-exec run --notify-command '<platform-shell-command>' -- echo done` is executed on Windows
When the job finishes
Then the configured command string is launched via the supported Windows shell path
And completion event delivery still uses stdin and the documented environment variables

### Requirement: notification failure does not change job result

Failure to deliver a completion notification must not change the job's terminal state or result (MUST). Notification failure must be recorded as a delivery result (MUST). `run`, `status`, and `wait` must not emit additional stdout output that breaks their JSON contract (MUST).

#### Scenario: command sink failure keeps exited state

Given `agent-exec run --notify-command 'nonexistent-command-for-agent-exec-test' -- echo done` is executed
When the main job succeeds but notification delivery fails
Then `status` returns `state=exited`
And `completion_event.json` records the notification failure against the configured command string
