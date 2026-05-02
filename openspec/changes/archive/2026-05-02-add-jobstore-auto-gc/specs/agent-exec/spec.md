## MODIFIED Requirements

### Requirement: run/start は既定で inline output を返す

`run` と `start` は既定で最大 10 秒待機し、待機中に観測できた stdout/stderr を inline で返さなければならない（MUST）。`--no-wait` 指定時は待機せず即時返却しなければならない（MUST）。レスポンスには `waited_ms`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を含めなければならない（MUST）。

`run` と `start` は、成功したジョブ起動パスで bounded auto-GC を best-effort に実行し、既定では 30 日より古い terminal job directory を削除候補にしなければならない（MUST）。auto-GC の失敗、lock 競合、budget 超過、個別 job の読み取り不能、または個別削除失敗は、親 `run` / `start` コマンドを失敗させてはならない（MUST NOT）。auto-GC は `running` と `created` の job directory を削除してはならない（MUST NOT）。

#### Scenario: run は既定で inline output を返す

Given `agent-exec run -- <cmd>` を実行する
When コマンドが返る
Then レスポンスに `job_id` と `state` とログパスが含まれる
And `waited_ms` と `stdout`/`stderr` が含まれる
And `stdout_range`/`stderr_range` と `stdout_total_bytes`/`stderr_total_bytes` が含まれる

#### Scenario: run performs bounded auto-GC without changing response contract

**Given**: root 配下に 30 日より古い terminal job directory が存在する
**When**: `agent-exec run -- echo hi` を実行する
**Then**: run レスポンスは JSON-only で既存の inline output fields を含む
**And**: 古い terminal job directory は削除される
**And**: 新しく起動した job directory は削除されない

#### Scenario: start performs bounded auto-GC without changing response contract

**Given**: root 配下に created job と 30 日より古い terminal job directory が存在する
**When**: `agent-exec start <created-job-id>` を実行する
**Then**: start レスポンスは JSON-only で既存の inline output fields を含む
**And**: 古い terminal job directory は削除される
**And**: start 対象 job は削除されず inspect 可能である

#### Scenario: auto-GC failure does not fail the parent command

**Given**: root 配下に読み取り不能または malformed な job-like directory が存在する
**When**: `agent-exec run -- echo hi` または `agent-exec start <job_id>` を実行する
**Then**: 親コマンドは成功レスポンスを返す
**And**: 問題のある directory は auto-GC によって削除成功として扱われない

### Requirement: delete と gc の削除結果可観測性

`delete` と `gc` は job directory を削除したと報告する場合、`remove_dir_all` が成功しただけでは不十分であり、コマンド完了時点で対象 job directory が存在しないことを確認してから `action="deleted"` を返さなければならない（MUST）。削除呼び出し後も対象 path が存在する場合は、削除成功として扱ってはならない（MUST NOT）。

`delete --all` は bulk delete の評価に使った effective cwd scope をレスポンスに含めなければならない（MUST）。`gc` と `delete` のレスポンスは、利用者が少なくとも「削除成功」「対象内だが削除されなかった」「対象外または条件不一致」を識別できるだけの action/reason または集計情報を含まなければならない（MUST）。

`gc` は retention window に加えて、terminal job の保持件数上限と root byte 上限を表す cleanup policy を受け付けられなければならない（MUST）。`gc --dry-run` は削除候補と root summary を返し、filesystem を変更してはならない（MUST NOT）。`gc` は summary として少なくとも scanned/job/non-job directory counts、state 別 counts、bytes、candidate/deleted/skipped/failed counts を返さなければならない（MUST）。

#### Scenario: gc returns deleted only after the directory is gone

**Given**: retention 条件を満たす terminal job directory が存在する
**When**: `agent-exec gc --older-than 7d` を実行する
**Then**: レスポンスでその job に `action="deleted"` を返すのは command 完了時点で当該 directory が存在しない場合だけである
**And**: command 完了時点でも directory が存在する場合は `action="deleted"` を返さない

#### Scenario: gc dry-run reports summary without deleting

**Given**: root 配下に terminal, running, created, unknown job directory が混在している
**When**: `agent-exec gc --dry-run --older-than 7d` を実行する
**Then**: レスポンスは root summary counts と deletion candidate details を含む
**And**: `would_delete` の job directory は command 完了時点でまだ存在する

#### Scenario: gc applies max job count cleanup to terminal jobs only

**Given**: root 配下に retention window 内外の terminal jobs と running jobs が存在する
**When**: `agent-exec gc --max-jobs 10 --dry-run` を実行する
**Then**: newest 10 件を超える terminal jobs だけが count pressure の削除候補になる
**And**: running jobs は件数圧力による削除候補にならない

#### Scenario: gc applies max byte cleanup to terminal jobs only

**Given**: root 全体の bytes が `--max-bytes` を超えている
**When**: `agent-exec gc --max-bytes <BYTES> --dry-run` を実行する
**Then**: 古い terminal jobs が byte pressure の削除候補として返る
**And**: running jobs と created jobs は byte pressure の削除候補にならない
