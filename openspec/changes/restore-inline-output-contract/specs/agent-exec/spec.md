## MODIFIED Requirements

### Requirement: run のジョブ生成と初回 inline output

`run` はジョブを起動し、既定では `--wait --until 10` 相当の待機予算内で観測できた stdout / stderr を初回レスポンスに含めなければならない（MUST）。`--no-wait` は `--wait --until 0` のエイリアスであり、追加待機なしの launch-only 返却を明示的に選べなければならない（MUST）。

`run` の出力は top-level の `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` で表現しなければならない（MUST）。range は raw byte offset の `[begin, end]` 配列で、意味は half-open interval `[begin, end)` とする（MUST）。

#### Scenario: run 既定は最大 10 秒待機して head を返す

Given `agent-exec run -- sh -c "printf 'hello'"` を実行する
When `run` の JSON が返る
Then `state` は終端状態である
And `stdout` は `hello` を含む
And `stdout_range` は `[0, 5]` である
And `stdout_total_bytes` は `5` である

#### Scenario: run --no-wait は待機なしで返る

Given `agent-exec run --no-wait -- sh -c "sleep 60"` を実行する
When `run` の JSON が返る
Then `waited_ms` は 0 近傍である
And ジョブは継続実行してよい

### Requirement: tail は range 付き末尾観測 API

`tail` はログ末尾の観測を担い、`stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を返さなければならない（MUST）。`tail` の range は返却した末尾部分の raw byte 区間を示さなければならない（MUST）。

#### Scenario: tail は末尾の range を返す

Given stdout.log が 1000 bytes あり、最後の 120 bytes が取得対象である
When `agent-exec tail <job_id>` を実行する
Then `stdout_range` は `[880, 1000]` である
And `stdout_total_bytes` は `1000` である

### Requirement: run/start/tail は range 契約を共有する

`run`, `start`, `tail` が返す stdout / stderr 本文は、同じ field 名と range 契約を共有しなければならない（MUST）。`snapshot`, `final_snapshot`, `truncated`, `stdout_tail`, `stderr_tail`, `stdout_observed_bytes`, `stderr_observed_bytes`, `stdout_included_bytes`, `stderr_included_bytes` を canonical field 名として返してはならない（MUST NOT）。

#### Scenario: canonical output fields are unified

Given `run`, `start`, `tail` の各レスポンスを比較する
When 本文と byte 範囲フィールドを確認する
Then 3 つとも `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を使う
And 削除済み field 名は含まれない
