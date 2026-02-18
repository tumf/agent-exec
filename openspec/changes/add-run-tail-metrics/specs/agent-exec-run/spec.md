# agent-exec-run Specification (Delta)

## MODIFIED Requirements

### Requirement: run の監視分離

`run` は `snapshot-after` の待機時間を最大 10,000ms に制限しなければならない（MUST）。
`run` の JSON には待機時間の実測値 `waited_ms` と `run` 呼び出し全体の所要時間 `elapsed_ms` を含めなければならない（MUST）。

#### Scenario: snapshot-after の上限

Given `agent-exec run --snapshot-after 15000 --max-bytes 64 -- <cmd>` を実行する
When 10 秒経過する
Then `waited_ms` は 10,000 以下であり、`elapsed_ms` は `waited_ms` 以上である

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
