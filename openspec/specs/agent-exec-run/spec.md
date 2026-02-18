# agent-exec-run Specification

## Purpose
TBD - created by archiving change define-agent-exec-run-supervise-v0-1. Update Purpose after archive.
## Requirements
### Requirement: run の監視分離

`run` は `snapshot-after` の待機時間を最大 10,000ms に制限しなければならない（MUST）。
`run` の JSON には待機時間の実測値 `waited_ms` と `run` 呼び出し全体の所要時間 `elapsed_ms` を含めなければならない（MUST）。

#### Scenario: snapshot-after の上限

Given `agent-exec run --snapshot-after 15000 --max-bytes 64 -- <cmd>` を実行する
When 10 秒経過する
Then `waited_ms` は 10,000 以下であり、`elapsed_ms` は `waited_ms` 以上である

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

`run` と `tail` の JSON には `stdout_log_path` と `stderr_log_path` を含めなければならない（MUST）。
`run` の `snapshot` および `tail` は、`stdout_observed_bytes`/`stderr_observed_bytes` と
`stdout_included_bytes`/`stderr_included_bytes` を含めなければならない（MUST）。
`observed_bytes` は取得時点のログファイルサイズ（bytes）を示し、
`included_bytes` は JSON に含めた `*_tail` の UTF-8 bytes 長を示す（MUST）。

#### Scenario: tail のログパスと bytes メトリクス

Given `agent-exec tail <job_id> --max-bytes 128` を実行する
When ログ末尾が取得される
Then `stdout_log_path` と `stderr_log_path` が含まれ、
`stdout_observed_bytes` と `stderr_observed_bytes` が 0 以上の整数で返る
And `stdout_included_bytes` と `stderr_included_bytes` が返り、`*_included_bytes` は `*_observed_bytes` を超えない

### Requirement: run の既定スナップショットと出力含有

`run` は既定で `snapshot` を返さなければならない（MUST）。既定の待機時間は `snapshot-after=200ms` 相当とし、`snapshot` の `stdout_tail`/`stderr_tail` は `tail-lines` と `max-bytes` の制約に従って末尾を含めなければならない（MUST）。`snapshot-after=0` のときは従来どおり `snapshot` を省略してよい（MAY）。

#### Scenario: 既定 run で stdout が含まれる

Given `agent-exec run -- echo hello` を実行する
When `run` の JSON が返る
Then `snapshot` が存在する
And `snapshot.stdout_tail` に `hello` が含まれる
And `snapshot.stdout_included_bytes` は `snapshot.stdout_observed_bytes` を超えない

### Requirement: 改行なし出力の捕捉

`stdout.log` と `stderr.log` は各ストリームの出力バイト列をそのまま追記保存しなければならない（MUST）。`run` の `snapshot` は改行の有無に関わらず `stdout`/`stderr` の末尾を含めなければならない（MUST）。`full.log` の行形式（`<RFC3339> [STDOUT|STDERR] <line>`）は維持する（MUST）。

#### Scenario: 改行なし stdout でも snapshot に含まれる

Given `agent-exec run --snapshot-after 200 --max-bytes 64 -- sh -c "printf 'abc'"` を実行する
When `run` の JSON が返る
Then `snapshot.stdout_tail` に `abc` が含まれる

