## MODIFIED Requirements

### Requirement: run/start の観測責務

`run` と `start` は launch-only ではなく、既定では `--wait --until 10` 相当の待機予算内で初回レスポンスに inline output を含めなければならない（MUST）。`--no-wait` は `--wait --until 0` のエイリアスとして受け付けなければならない（MUST）。

`run` / `start` の `stdout` と `stderr` は、それぞれのログの先頭 `N` bytes を UTF-8 lossy で復元した文字列でなければならない（MUST）。`stdout_range[0]` と `stderr_range[0]` は 0 でなければならない（MUST）。

#### Scenario: start 既定は初回 head を返す

Given `agent-exec create -- sh -c "printf 'abc'"` で作成した job がある
When `agent-exec start <job_id>` を実行する
Then `start` の JSON は `stdout` に `abc` を含む
And `stdout_range` は `[0, 3]` である
And `stdout_total_bytes` は `3` である

#### Scenario: start --no-wait は launch-only を明示選択する

Given `agent-exec create -- sh -c "sleep 60"` で作成した job がある
When `agent-exec start --no-wait <job_id>` を実行する
Then `start` の JSON は追加待機なしに返る
And ジョブは継続実行してよい

### Requirement: tail が range 付き末尾観測を担う

`tail` はログ末尾の観測を担い、`stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を返さなければならない（MUST）。`run` / `start` の head 契約と field 名は共有するが、返却する byte 区間は末尾側でなければならない（MUST）。

#### Scenario: tail が末尾 API である

Given 実行中または完了済みのジョブが存在する
When `agent-exec tail <job_id> --tail-lines 10 --max-bytes 1024` を実行する
Then `stdout` / `stderr` と range 情報が返る
And `stdout_range[1]` は `stdout_total_bytes` 以下である
And range から返却内容が末尾側であることを判定できる

### Requirement: run/status/tail/wait/kill の JSON

`run`, `start`, `tail` の JSON には `stdout_log_path` と `stderr_log_path` を含めなければならない（MUST）。`run` / `start` / `tail` が本文を返す場合、canonical field は `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` でなければならない（MUST）。削除済み snapshot-era field 名を新契約として返してはならない（MUST NOT）。

#### Scenario: run は inline output とログパスを返す

Given `agent-exec run -- git diff --staged` を実行する
When `run` の JSON が返る
Then `stdout` と `stdout_range` と `stdout_total_bytes` が含まれる
And `stdout_log_path` と `stderr_log_path` が含まれる
And `snapshot` と `final_snapshot` は含まれない
