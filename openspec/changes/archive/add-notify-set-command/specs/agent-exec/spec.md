## ADDED Requirements

### Requirement: notify set updates persisted notification metadata

`agent-exec` must provide a `notify set` subcommand that updates the persisted notification metadata for an existing job (MUST). The subcommand must accept a job identifier and a `--command <COMMAND>` shell command string (MUST). On success it must return a JSON success envelope describing the saved notification configuration (MUST).

#### Scenario: update command notification for an existing job

Given an existing job with identifier `<job_id>`
When `agent-exec notify set <job_id> --command 'cat >/tmp/event.json'` is executed
Then stdout is a single JSON success object
And `meta.json.notification.notify_command` for `<job_id>` equals `cat >/tmp/event.json`

### Requirement: notify set is metadata-only for any job state

`notify set` must be accepted for any existing job state (MUST). Executing `notify set` must not immediately execute the configured notification command (MUST), even when the target job is already terminal.

#### Scenario: terminal job metadata update does not trigger delivery

Given an existing job `<job_id>` is already in a terminal state
When `agent-exec notify set <job_id> --command 'cat >/tmp/event.json'` is executed
Then the command succeeds
And no notification command is executed as part of `notify set`

#### Scenario: missing job returns job_not_found

Given no job exists for identifier `<job_id>`
When `agent-exec notify set <job_id> --command 'cat >/tmp/event.json'` is executed
Then stdout is a single JSON error object
And `error.code` equals `job_not_found`
