## MODIFIED Requirements

### Requirement: run completion notification configuration

`run` must support persisted notification sinks for both job completion and output matches (MUST). Completion delivery must continue to consult the latest persisted notification metadata at dispatch time rather than assuming launch-time values are still current (MUST). When output-match notification metadata is present, the running supervisor must consult the latest persisted settings for newly observed stdout/stderr lines and emit `job.output.matched` events for matching future lines (MUST).

#### Scenario: output-match settings added by notify set affect future output only

Given `agent-exec run -- sh -c "echo before; sleep 1; echo ERROR"` creates job `<job_id>`
And `agent-exec notify set <job_id> --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json'` is executed while `<job_id>` is still running
When the later `ERROR` line is emitted
Then a `job.output.matched` notification is delivered for `<job_id>`
And the earlier `before` line is not replayed as a notification

### Requirement: command sink and file sink delivery contract

The command sink for notification events must execute through the platform shell and receive event JSON on stdin (MUST). `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and `AGENT_EXEC_EVENT_TYPE` must be added to the sink environment for both `job.finished` and `job.output.matched` delivery (MUST). File sinks must append one event JSON line as NDJSON and create parent directories when needed (MUST).

#### Scenario: output-match command sink receives event metadata

Given a running job has persisted output-match settings with a command sink
When a stdout or stderr line matches the configured output pattern
Then the shell command receives a `job.output.matched` event JSON payload on stdin
And `AGENT_EXEC_EVENT_TYPE=job.output.matched` is present in the sink environment

#### Scenario: output-match file sink appends one line per match

Given a running job has persisted output-match settings with a file sink
When two later lines match the configured output pattern
Then the output-match event file contains two additional NDJSON lines

### Requirement: notification failure does not change job result

Failure to deliver a completion notification or an output-match notification must not change the job's terminal state or result (MUST). Notification failure must be recorded as a delivery result (MUST). `run`, `status`, and `wait` must not emit additional stdout output that breaks their JSON contract (MUST).

#### Scenario: output-match sink failure keeps job state unchanged

Given a running job has persisted output-match settings whose command sink exits non-zero
When a later line matches the configured output pattern
Then notification delivery is recorded as failed
And the job continues with its normal lifecycle state transitions unaffected
