## MODIFIED Requirements

### Requirement: notify set updates persisted notification metadata

`agent-exec` must provide a `notify set` subcommand that updates persisted notification metadata for an existing job (MUST). In addition to the completion-oriented `--command <COMMAND>` path, the subcommand must accept output-match configuration that persists an output pattern, match mode, stream selector, and command/file sinks without executing them immediately (MUST). On success it must return a JSON success envelope describing the saved notification configuration (MUST).

#### Scenario: notify set saves output-match configuration for an existing job

Given an existing job with identifier `<job_id>`
When `agent-exec notify set <job_id> --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json'` is executed
Then stdout is a single JSON success object
And `meta.json.notification` for `<job_id>` stores the configured output-match pattern and sink

### Requirement: notify set is metadata-only for any job state

`notify set` must be accepted for any existing job state (MUST). Executing `notify set` must not immediately execute completion notification sinks or output-match notification sinks (MUST), even when the target job is already terminal.

#### Scenario: terminal job output-match update does not trigger delivery

Given an existing job `<job_id>` is already in a terminal state
When `agent-exec notify set <job_id> --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json'` is executed
Then the command succeeds
And no notification command is executed as part of `notify set`

#### Scenario: missing job returns job_not_found for output-match updates

Given no job exists for identifier `<job_id>`
When `agent-exec notify set <job_id> --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json'` is executed
Then stdout is a single JSON error object
And `error.code` equals `job_not_found`
