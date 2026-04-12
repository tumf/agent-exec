## MODIFIED Requirements

### Requirement: snapshot/tail の責務分離

ログ末尾の観測は `tail` が担わなければならない（MUST）。`run` と `start` は snapshot を返してはならず（MUST NOT）、`tail-lines` と `max-bytes` による切り詰め、および `encoding="utf-8-lossy"` を伴う末尾取得契約は `tail` 専用としなければならない（MUST）。

#### Scenario: tail が末尾観測 API である
Given 実行中または完了済みのジョブがある
When `agent-exec tail <job_id> --lines 10 --max-bytes 1024` を実行する
Then `stdout_tail`/`stderr_tail` は制約内の内容であり `encoding` が含まれる
And `run` / `start` のレスポンスには同等の snapshot フィールドは含まれない

### Requirement: run/status/tail/wait/kill の JSON

`run` は `job_id`、`state`、`stdout_log_path`、`stderr_log_path` を含む JSON を返さなければならない（MUST）。`run` は `snapshot`、`final_snapshot`、snapshot 由来の bytes メトリクスを含めてはならない（MUST NOT）。`tail` は `stdout_log_path` と `stderr_log_path` と bytes メトリクスを含めなければならない（MUST）。`observed_bytes` は取得時点のログファイルサイズ（bytes）を示し、`included_bytes` は JSON に含めた `*_tail` の UTF-8 bytes 長を示す（MUST）。

#### Scenario: run はログパスだけを返す

Given `agent-exec run -- echo hi` を実行する
When `run` の JSON が返る
Then `stdout_log_path` と `stderr_log_path` が含まれる
And `snapshot` は含まれない
And `final_snapshot` は含まれない

#### Scenario: tail はログパスと bytes メトリクスを返す

Given `agent-exec tail <job_id> --max-bytes 128` を実行する
When ログ末尾が取得される
Then `stdout_log_path` と `stderr_log_path` が含まれ、
And `stdout_observed_bytes` と `stderr_observed_bytes` が 0 以上の整数で返る
And `stdout_included_bytes` と `stderr_included_bytes` が返り、`*_included_bytes` は `*_observed_bytes` を超えない

### Requirement: run と start の観測責務削除

`run` と `start` はジョブ起動コマンドとして即時返却しなければならない（MUST）。完了待機は `wait` が担い、出力取得は `tail` が担わなければならない（MUST）。`run --wait` と `start --wait`、および snapshot 系オプションは受け付けてはならない（MUST NOT）。

#### Scenario: start は snapshot なしで即時返却する

Given `agent-exec create -- sh -c "sleep 1; echo hi"` で作成した job がある
When `agent-exec start <job_id>` を実行する
Then `start` の JSON に `job_id` と初期 state が含まれる
And `snapshot` は含まれない
And `final_snapshot` は含まれない
And 後続の `agent-exec wait <job_id>` と `agent-exec tail <job_id>` で完了待機と出力取得が行える

#### Scenario: start は削除済み観測オプションを拒否する

Given `agent-exec start --snapshot-after 10 <job_id>` を実行する
When CLI 引数を検証する
Then usage error で失敗する

And given `agent-exec start --wait <job_id>` を実行する
When CLI 引数を検証する
Then usage error で失敗する
