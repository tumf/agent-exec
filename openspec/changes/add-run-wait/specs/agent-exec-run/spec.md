# agent-exec-run Spec Delta (add-run-wait)

## MODIFIED Requirements
### Requirement: run の同期待機オプション

`run` は `--wait` が指定された場合、ジョブが終端状態 (`exited|killed|failed`) になるまで待機しなければならない（MUST）。`--wait` 指定時、`snapshot-after` の待機上限 (10,000ms) を適用してはならない（MUST）。
`--wait` 指定時の `run` JSON は `exit_code`（存在する場合）と `finished_at` を含めなければならない（MUST）。
`--wait` 指定時の `run` JSON は終了時点のログ末尾を示す `final_snapshot` を含めなければならない（MUST）。`final_snapshot` の構造と制約は既存の `snapshot` と同一でなければならない（MUST）。
`--wait` 指定時の `waited_ms` は終端状態までの待機時間を示さなければならない（MUST）。

#### Scenario: --wait で終了まで待機する

Given `agent-exec run --wait -- sh -c "echo hi"` を実行する
When `run` の JSON が返る
Then `state` は `exited` である
And `final_snapshot.stdout_tail` に `hi` が含まれる
And `finished_at` が含まれる
