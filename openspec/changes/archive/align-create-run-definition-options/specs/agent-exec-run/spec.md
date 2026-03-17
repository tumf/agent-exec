## MODIFIED Requirements

### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。

`run` が受け付ける definition-time option は、同じ persisted job definition を表す限り `create` でも受け付けなければならない（MUST）。そのような option は `run` と `create` の両方で同じ `meta.json` 意味論に落ちるよう定義しなければならない（MUST）。一方で `snapshot-after`, tail 制約, `--wait` のような観測用 option は `run` 固有の launch/observation-time option として扱ってよい（MAY）。

#### Scenario: persisted env definition stays aligned between create and run

Given `--env-file A --env KEY=VALUE` is part of the persisted job definition
When a job is created via `agent-exec create` and another equivalent job is created via `agent-exec run`
Then both jobs persist equivalent environment-definition metadata
And any difference between the commands is limited to immediate execution behavior

### Requirement: create initial tag assignment

`create` must accept repeatable `--tag <TAG>` using the same validation and deduplication rules as `run` (MUST). The persisted tags must be available to `start` without requiring any additional tag mutation command (MUST).

#### Scenario: create stores deduplicated tags

Given `agent-exec create --tag aaa --tag bbb --tag aaa -- sh -c "echo hi"` is executed
When the job metadata is written
Then the persisted tags are `["aaa", "bbb"]`
And a later `agent-exec start <job_id>` uses those tags as the job's initial tag set

### Requirement: run completion notification configuration

`run` must support persisted notification sinks for both job completion and output matches (MUST). Completion delivery must continue to consult the latest persisted notification metadata at dispatch time rather than assuming launch-time values are still current (MUST). When output-match notification metadata is present, the running supervisor must consult the latest persisted settings for newly observed stdout/stderr lines and emit `job.output.matched` events for matching future lines (MUST).

Notification settings are definition-time metadata and therefore must be accepted by both `create` and `run` (MUST). Jobs defined through either path must persist the same notification metadata shape before execution begins (MUST).

#### Scenario: create and run persist the same notification metadata

Given `--notify-command 'cat >/tmp/event.json' --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json'` is provided as job-definition input
When one job is defined with `agent-exec create` and another with `agent-exec run`
Then both jobs persist equivalent notification metadata
And only the `run` path begins execution immediately

#### Scenario: create persists output-match notifications for later start

Given `agent-exec create --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json' -- sh -c "echo ERROR"` is executed
When `agent-exec start <job_id>` later launches that created job
Then the running job uses the output-match notification settings saved during `create`
And `create` itself did not trigger any notification delivery
