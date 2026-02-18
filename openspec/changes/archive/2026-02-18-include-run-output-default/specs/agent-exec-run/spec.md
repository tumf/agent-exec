# agent-exec-run 変更仕様: include-run-output-default

## ADDED Requirements

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
