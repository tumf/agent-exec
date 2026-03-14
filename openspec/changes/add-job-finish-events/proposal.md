# 変更提案: job completion event hook を追加する

## Problem/Context

`agent-exec` は detached job runner として有用ですが、外部オーケストレータがジョブ完了を検知するには `status` / `tail` / `wait` を polling する必要があります。
`run --wait` は短命ジョブには有効なものの、呼び出し元セッションより長く生きるジョブでは push 型の完了通知を置き換えられません。
既存の `meta.json` / `state.json` / `stdout.log` / `stderr.log` には完了通知に必要な情報の大半が既に保存されているため、MVP は terminal state 到達後に再利用可能な completion event を配送する仕組みを追加するのが最小です。

## Proposed Solution

`run` に completion notification 設定を追加し、supervisor がジョブを terminal state (`exited|killed|failed`) に更新した直後に `job.finished` イベントを 1 回だけ配送できるようにします。

MVP の sink は以下に限定します。

- `--notify-command <json-argv>`: JSON 配列で与えた argv を shell を介さず実行し、event JSON を stdin で渡す
- `--notify-file <path>`: completion event を NDJSON で追記する

通知設定は `meta.json` に保存し、配送結果はジョブディレクトリ配下の `completion_event.json` に記録します。通知の失敗は job 本体の `state` / `exit_code` を変更せず、後続の監査と再送判断のための delivery result のみを残します。

## Acceptance Criteria

- `agent-exec run` は `--notify-command` と `--notify-file` を受け付け、通知設定をジョブ metadata に永続化できる
- supervisor は terminal state を `state.json` に反映した後、`job.finished` イベントを 1 回だけ生成して各 sink へ配送する
- event payload は `job_id`, `state`, `command`, `cwd`, `started_at`, `finished_at`, `duration_ms`, `exit_code`, `signal`, `stdout_log_path`, `stderr_log_path` を含む
- `--notify-command` は shell を介さず argv を実行し、event JSON を stdin と環境変数で受け取れる
- `--notify-file` は指定ファイルに event JSON を 1 行追記し、親ディレクトリがなければ作成する
- 通知失敗時も `run` / `status` / `wait` の既存 JSON 契約と job result は維持される
- 統合テストで command sink / file sink / notification failure 非破壊性を検証する

## Out of Scope

- `webhook` sink と HTTP retry / backoff
- user-defined free-form metadata や labels の拡張
- delivery retry queue や永続的な再送ワーカー
- 既存 `run` / `wait` の stdout JSON 形状の大幅な変更
