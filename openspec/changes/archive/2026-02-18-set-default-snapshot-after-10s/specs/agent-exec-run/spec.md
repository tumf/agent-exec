# agent-exec-run Specification (Delta)

## MODIFIED Requirements

### Requirement: run の既定スナップショットと出力含有

`run` は既定で `snapshot` を返さなければならない（MUST）。既定の待機時間は `snapshot-after=10000ms` 相当とし、`snapshot` の `stdout_tail`/`stderr_tail` は `tail-lines` と `max-bytes` の制約に従って末尾を含めなければならない（MUST）。`snapshot-after=0` のときは従来どおり `snapshot` を省略してよい（MAY）。

#### Scenario: 既定 run は最大 10 秒待機する

Given `agent-exec run -- ping localhost` を実行する
When `run` の JSON が返る
Then `snapshot` が存在する
And `waited_ms` は 10,000 以下である
