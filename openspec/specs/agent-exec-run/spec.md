# agent-exec-run Specification

## Purpose
TBD - created by archiving change define-agent-exec-run-supervise-v0-1. Update Purpose after archive.
## Requirements
### Requirement: run の監視分離

`run` は `snapshot-after` 経過時点で JSON を返して終了しなければならない（MUST）。その後の stdout/stderr 収集と `state.json` 更新は監視プロセスに引き継がれなければならない（MUST）。

#### Scenario: snapshot-after での戻り
Given `agent-exec run --snapshot-after 2s -- <cmd>` を実行する
When 2 秒経過する
Then stdout に `type="run"` を含む JSON が 1 回出力され、`run` は終了する

### Requirement: snapshot/tail の末尾取得

`run` の `snapshot` と `tail` は `stdout.log`/`stderr.log` の末尾から生成しなければならない（MUST）。`tail-lines` と `max-bytes` の両制約で切り詰め、`encoding="utf-8-lossy"` を返さなければならない（MUST）。

#### Scenario: tail の制約適用
Given `agent-exec tail <job_id> --lines 10 --max-bytes 1024` を実行する
When ログ末尾が取得される
Then `stdout_tail`/`stderr_tail` は制約内の内容であり `encoding` が含まれる

### Requirement: ログファイル

`stdout.log` と `stderr.log` はそれぞれのストリームを追記保存しなければならない（MUST）。`full.log` は時刻とストリーム種別を含む 1 行形式で追記しなければならない（MUST）。

#### Scenario: full.log の形式
Given 実行中のジョブがある
When `full.log` が追記される
Then 各行は `RFC3339 timestamp` と `[STDOUT]` または `[STDERR]` を含む

### Requirement: timeout と kill-after

`--timeout` が指定された場合、期限到達時に終了シグナルを送信し、`--kill-after` 経過後も生存している場合は強制終了しなければならない（MUST）。

#### Scenario: timeout の強制終了
Given `agent-exec run --timeout 1s --kill-after 1s -- <cmd>` を実行する
When 2 秒経過する
Then 対象プロセスは終了している

### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。

#### Scenario: env の上書き
Given `--env-file A --env-file B --env KEY=VALUE` を指定する
When 環境が構築される
Then `KEY` は `--env` の値で上書きされる

### Requirement: mask の適用範囲

`--mask KEY` は JSON 出力および `meta.json` の表示にのみ適用され、実際のプロセス環境は変更してはならない（MUST）。

#### Scenario: mask の表示
Given `--env SECRET=aaa --mask SECRET` を指定する
When `run` の JSON が返る
Then `SECRET` の値はマスクされて表示される

### Requirement: log パスの指定

`--log <path>` が指定された場合、`full.log` の保存先はそのパスでなければならない（MUST）。未指定の場合はジョブディレクトリ配下の `full.log` としなければならない（MUST）。

#### Scenario: log パスの上書き
Given `agent-exec run --log /tmp/agent.log -- <cmd>` を実行する
When ログが書き込まれる
Then `/tmp/agent.log` に `full.log` が保存される

### Requirement: progress-every の扱い

`--progress-every` が指定された場合、監視プロセスはその間隔以内に `state.json.updated_at` を更新しなければならない（MUST）。stdout に追加の JSON を出力してはならない（MUST）。

#### Scenario: progress 更新
Given `agent-exec run --progress-every 5s -- <cmd>` を実行する
When 5 秒経過する
Then `state.json.updated_at` が更新されている

### Requirement: run/status/tail/wait/kill の JSON

`run`/`status`/`tail`/`wait`/`kill` はそれぞれの出力に必要なフィールドを含まなければならない（MUST）。

#### Scenario: tail の JSON
Given `agent-exec tail <job_id>` を実行する
When コマンドが完了する
Then `job_id`, `stdout_tail`, `stderr_tail`, `truncated`, `encoding` を含む

