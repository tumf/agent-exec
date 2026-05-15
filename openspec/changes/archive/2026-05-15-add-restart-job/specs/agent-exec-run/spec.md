## ADDED Requirements

### Requirement: restart launch semantics

`restart` MUST launch an existing job from its persisted job definition using the same supervisor path as `start`, while allowing current states `created`, `running`, `exited`, `killed`, and `failed` when the persisted definition is usable.

#### Scenario: restart launches from meta command

**Given**: an existing job has `meta.json.command` set to a command that prints `restart-ok`
**When**: `agent-exec restart <job_id> --wait` is executed
**Then**: the launched child process runs the command from `meta.json`
**And**: the restart response stdout includes `restart-ok`

### Requirement: restart preserves launch-time option semantics

Restart MUST apply persisted runtime controls and observation controls consistently with `start`. Runtime controls stored in metadata, such as timeout, kill-after, progress-every, stdin file, notification settings, environment settings, and shell wrapper, MUST apply to the restarted process. Observation controls passed to `restart`, such as `--wait`, `--until`, `--forever`, `--no-wait`, and `--max-bytes`, MUST affect only the restart response observation.

#### Scenario: restart honors persisted timeout

**Given**: a job definition has a persisted timeout that is shorter than its command runtime
**When**: `agent-exec restart <job_id> --wait --forever` is executed
**Then**: the restarted process is terminated by the persisted timeout
**And**: the response eventually reports a terminal state

#### Scenario: restart honors response no-wait without changing runtime

**Given**: a restartable job command sleeps for several seconds
**When**: `agent-exec restart --no-wait <job_id>` is executed
**Then**: restart returns promptly
**And**: the process continues running unless stopped by persisted runtime controls
