## ADDED Requirements

### Requirement: create and start lifecycle commands

`agent-exec` MUST support a two-step lifecycle in addition to immediate `run`. `create` MUST persist a job definition without launching the command, and `start <job_id>` MUST launch a previously created job using the persisted definition.

#### Scenario: create persists without execution

Given `agent-exec create -- sh -c "echo hi > marker"` is executed
When the command returns
Then stdout contains a JSON object with `type="create"` and `state="created"`
And the job directory contains `meta.json`, `state.json`, `stdout.log`, `stderr.log`, and `full.log`
And the command has not yet been executed

#### Scenario: start launches a created job

Given a job exists in `state="created"`
When `agent-exec start <job_id>` is executed
Then stdout contains a JSON object with `type="start"`
And the job transitions to `running` or a terminal state according to the existing snapshot / wait rules

#### Scenario: start rejects already-started jobs

Given a job exists in `state="running"`, `state="exited"`, `state="killed"`, or `state="failed"`
When `agent-exec start <job_id>` is executed
Then stdout contains `ok=false`
And the error code reports an invalid lifecycle transition

## MODIFIED Requirements

### Requirement: list の状態フィルタ

`list` は `--state <state>` を受け付け、指定時は `jobs` を `jobs[].state == <state>` に一致するものだけ返さなければならない（MUST）。
`state` の値は `created|running|exited|killed|failed|unknown` に限定され、未知の値は usage エラーとする（MUST）。
`--state` 指定時はフィルタ適用後の件数に対して `--limit` を適用し、必要に応じて `truncated=true` としなければならない（MUST）。

#### Scenario: 未開始ジョブのみの取得

Given `created` のジョブと `running` のジョブが存在する
When `agent-exec list --state created` を実行する
Then `jobs` は `state=created` のジョブのみを含む
And `jobs` の全要素で `state` は `created` である
