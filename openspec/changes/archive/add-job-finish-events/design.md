# 技術設計: job completion event hook

## 目的

polling を前提にしない完了通知を `agent-exec` に追加し、外部オーケストレータが long-running job の終端状態を push 型で受け取れるようにする。

## 設計方針

- **最小の sink から始める**: MVP は `command` と `file` のみを対象にし、`webhook` は別 proposal に分離する
- **既存契約を維持する**: stdout JSON-only と既存 `run` / `status` / `wait` のレスポンス形状は壊さない
- **terminal state を正とする**: completion event は `state.json` の terminal state 書き込み後に生成する
- **job result と delivery result を分離する**: 通知失敗で job 本体の `state` や `exit_code` を変更しない

## completion event model

supervisor は terminal state 到達後に単一の `job.finished` event payload を組み立てる。

payload には少なくとも以下を含める。

- `schema_version`
- `event_type`
- `job_id`
- `state`
- `command`
- `cwd`
- `started_at`
- `finished_at`
- `duration_ms`
- `exit_code`
- `signal`
- `stdout_log_path`
- `stderr_log_path`

MVP では sink ごとの入力を揃えるため、payload 本体は共通 JSON 1 つに固定し、job directory 配下の `completion_event.json` に保存する。

## sink semantics

### command sink

- 入力は `--notify-command '["/path/to/bin","arg1"]'` の JSON 配列とする
- shell を介さず `Command::new(argv[0]).args(&argv[1..])` で起動する
- event JSON 全体を stdin で渡す
- 補助情報として `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, `AGENT_EXEC_EVENT_TYPE=job.finished` を環境変数に追加する

### file sink

- 指定 path に event JSON を NDJSON で 1 行追記する
- 親ディレクトリがなければ作成する
- 同一 completion event に対して 1 行だけ書く

## persisted state

- `meta.json` に notification 設定を保存する
- `completion_event.json` に共通 payload と sink ごとの delivery result を保存する
- delivery result には sink 種別、対象、成功可否、失敗メッセージ、試行時刻を保持する

これにより、通知失敗時も後から監査・再送判断ができる。

## terminal-state sequencing

処理順は次のとおりとする。

1. child process 終了を検知する
2. `state.json` を terminal state に更新する
3. completion event payload を組み立てて `completion_event.json` に保存する
4. 各 sink に配送する
5. sink ごとの結果を `completion_event.json` に反映する

この順序により、通知の途中失敗があっても `status` / `wait` から見える job result は既に確定している。

## テスト方針

- file sink: 一時ファイルに NDJSON が 1 行追記され、`job_id` と terminal state が含まれることを検証する
- command sink: 補助スクリプトに stdin を保存させ、event JSON と追加 env を受け取れることを検証する
- failure semantics: 存在しない command などで sink を失敗させても job state が `failed` に改ざんされないことを検証する
