# agent-exec Specification (Delta)

## MODIFIED Requirements

### Requirement: run のジョブ生成とスナップショット

`run` は `snapshot-after` の待機時間を最大 10,000ms に制限しなければならない（MUST）。
`run` の `snapshot` は `stdout_observed_bytes`/`stderr_observed_bytes` と
`stdout_included_bytes`/`stderr_included_bytes` を含めなければならない（MUST）。

#### Scenario: snapshot の bytes メトリクス

Given `agent-exec run --snapshot-after 500 --max-bytes 64 -- <cmd>` を実行する
When snapshot が返る
Then `snapshot.stdout_observed_bytes` と `snapshot.stderr_observed_bytes` が含まれる
And `snapshot.stdout_included_bytes` と `snapshot.stderr_included_bytes` が含まれる
