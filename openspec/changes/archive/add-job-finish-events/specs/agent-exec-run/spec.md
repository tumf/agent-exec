# agent-exec-run Specification (Change: add-job-finish-events)

## ADDED Requirements

### Requirement: run の completion notification 設定

`run` は completion notification sink を設定できなければならない（MUST）。MVP では `--notify-command <json-argv>` と `--notify-file <path>` を受け付けなければならない（MUST）。`--notify-command` は shell 展開を前提とする単一文字列ではなく、argv を表す JSON 配列として解釈されなければならない（MUST）。notification 設定はジョブ metadata に永続化されなければならない（MUST）。

#### Scenario: command sink 設定を永続化する
Given `agent-exec run --notify-command '["/bin/sh","-c","cat >/tmp/event.json"]' -- echo hi` を実行する
When ジョブ metadata が作成される
Then `meta.json` には command sink の設定が保存される

### Requirement: terminal state 後に completion event を 1 回生成する

supervisor はジョブが terminal state (`exited|killed|failed`) に到達したとき、`state.json` を更新した後に `job.finished` completion event を 1 回だけ生成しなければならない（MUST）。event payload には `job_id`, `state`, `command`, `cwd`, `started_at`, `finished_at`, `duration_ms`, `exit_code`, `signal`, `stdout_log_path`, `stderr_log_path` を含めなければならない（MUST）。completion event は job directory 配下に保存されなければならない（MUST）。

#### Scenario: exited ジョブで completion event を保存する
Given `agent-exec run --notify-file /tmp/agent-exec-events.ndjson -- echo done` を実行する
When ジョブが `exited` になる
Then job directory に `completion_event.json` が作成される
And payload には `job_id`, `state=exited`, `finished_at`, `stdout_log_path`, `stderr_log_path` が含まれる

### Requirement: command sink と file sink の配送契約

`--notify-command` で指定した sink は shell を介さず実行され、completion event JSON を stdin で受け取らなければならない（MUST）。また `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, `AGENT_EXEC_EVENT_TYPE` の環境変数が追加されなければならない（MUST）。`--notify-file` で指定した sink は event JSON を NDJSON として 1 行追記し、親ディレクトリがなければ作成しなければならない（MUST）。

#### Scenario: file sink に NDJSON を追記する
Given `agent-exec run --notify-file /tmp/agent-exec/events.ndjson -- echo done` を実行する
When ジョブが完了する
Then `/tmp/agent-exec/events.ndjson` には completion event JSON が 1 行追記される

#### Scenario: command sink が event JSON と env を受け取る
Given `agent-exec run --notify-command '["/path/to/hook"]' -- echo done` を実行する
When ジョブが完了する
Then hook process は stdin から completion event JSON を受け取る
And `AGENT_EXEC_EVENT_TYPE=job.finished` が環境変数に含まれる

### Requirement: notification failure は job result を変更しない

completion notification sink の配送に失敗しても、job 本体の terminal state と result は変更されてはならない（MUST）。通知失敗は delivery result として記録されなければならない（MUST）。`run`, `status`, `wait` の stdout JSON 契約を壊す追加出力を行ってはならない（MUST）。

#### Scenario: command sink failure でも exited を維持する
Given `agent-exec run --notify-command '["/no/such/binary"]' -- echo done` を実行する
When ジョブ本体は正常終了し notification 配送が失敗する
Then `status` は job の `state=exited` を返す
And `completion_event.json` には notification failure が記録される
