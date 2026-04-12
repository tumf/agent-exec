## MODIFIED Requirements

### Requirement: run のジョブ生成と即時返却

`run` はジョブを起動し、観測用 snapshot を返却前に生成するための追加待機を行ってはならない（MUST NOT）。`run` の主責務は job 起動と `job_id` / 初期 state / ログパスの返却であり、完了待機と出力観測は `wait` / `tail` / `status` に分離しなければならない（MUST）。

`snapshot-after`、`tail-lines`、`max-bytes`、`wait` は `run` の CLI で受け付けてはならない（MUST NOT）。

#### Scenario: run は snapshot なしで即時返却する

Given `agent-exec run -- sh -c "sleep 1; echo hi"` を実行する
When `run` の JSON が返る
Then `job_id` が含まれる
And `snapshot` は含まれない
And `final_snapshot` は含まれない
And 後続の `agent-exec wait <job_id>` と `agent-exec tail <job_id>` で完了待機と出力取得が行える

#### Scenario: run は削除済み snapshot オプションを拒否する

Given `agent-exec run --snapshot-after 10 -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する

And given `agent-exec run --tail-lines 10 -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する

And given `agent-exec run --max-bytes 10 -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する

#### Scenario: run は削除済み wait オプションを拒否する

Given `agent-exec run --wait -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する
