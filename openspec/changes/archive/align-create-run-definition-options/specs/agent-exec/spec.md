## MODIFIED Requirements

### Requirement: create and start lifecycle commands

`agent-exec` MUST support a two-step lifecycle in addition to immediate `run`. `create` MUST persist a job definition without launching the command, and `start <job_id>` MUST launch a previously created job using the persisted definition.

For the job-definition portion of the lifecycle, `create` and `run` MUST accept the same definition-time options whenever those options contribute to persisted job metadata (MUST). `run` MAY additionally accept immediate-execution or observation-time options that `create` does not expose (MAY). `start` MUST consume the persisted definition rather than redefining those definition-time options (MUST).

This shared definition-time option surface MUST include persisted tags and persisted notification settings when those metadata families are supported (MUST). `create` MUST save those values without launching notification side effects, and `start` MUST use the saved values when launching the job (MUST).

#### Scenario: run and create share persisted definition inputs

Given a definition-time option contributes to `meta.json`
When that option is supported for `agent-exec run`
Then `agent-exec create` also accepts it unless the spec explicitly documents it as launch-only
And jobs created via `create` and via `run` persist the same metadata shape for that option

#### Scenario: create stores tags and notifications as shared definition metadata

Given `agent-exec create --tag aaa --notify-command 'cat >/tmp/event.json' -- sh -c "echo hi"` is executed
When the command returns
Then the job metadata stores tag `aaa` and the configured notification settings
And no notification command has been executed during `create`
