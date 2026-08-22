## MODIFIED Requirements

### Requirement: run のジョブ生成と inline output

`run` はジョブを起動し、既定で最大 10 秒待機して inline output を返さなければならない（MUST）。
`--no-wait` が指定された場合は即時返却しなければならない（MUST）。
`--wait`、`--until`、`--forever`、`--no-wait`、`--max-bytes` を受け付けなければならない（MUST）。
`run`/`create` の runtime 制御時間オプション（`--abandon-job-after`、`--kill-after`、`--progress-every`）は人間向け契約として秒単位で提示されなければならない（MUST）。private `_supervise` handoff は公開 surface ではなく、その秒単位 `--timeout` wire spelling を維持しなければならない（MUST）。

#### Scenario: run は既定待機で inline output を返す

Given `agent-exec run -- sh -c "sleep 1; echo hi"` を実行する
When `run` の JSON が返る
Then `job_id` が含まれる
And `waited_ms` と `stdout` が含まれる
And `stdout_range[0]` は `0` である

#### Scenario: run --no-wait は即時返却する

Given `agent-exec run --no-wait -- sh -c "sleep 1; echo hi"` を実行する
When `run` の JSON が返る
Then コマンドは追加待機せず返る
And `waited_ms` は短時間である
