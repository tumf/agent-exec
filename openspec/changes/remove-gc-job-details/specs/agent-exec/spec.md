## MODIFIED Requirements

### Requirement: delete と gc の削除結果可観測性

`delete` と `gc` は job directory を削除したと報告する場合、`remove_dir_all` が成功しただけでは不十分であり、コマンド完了時点で対象 job directory が存在しないことを確認してから削除成功として集計しなければならない（MUST）。削除呼び出し後も対象 path が存在する場合は、削除成功として扱ってはならない（MUST NOT）。

`delete --all` は bulk delete の評価に使った effective cwd scope をレスポンスに含めなければならない（MUST）。`delete` のレスポンスは、利用者が少なくとも「削除成功」「対象内だが削除されなかった」「対象外または条件不一致」を識別できるだけの action/reason または集計情報を含まなければならない（MUST）。`gc` のレスポンスは per-job `jobs` details を返してはならず（MUST NOT）、summary counters だけで「削除成功」「対象内だが削除されなかった」「対象外または条件不一致」を識別できなければならない（MUST）。

`gc` は retention window に加えて、terminal job の保持件数上限と root byte 上限を表す cleanup policy を受け付けられなければならない（MUST）。`gc --dry-run` は root summary を返し、filesystem を変更してはならない（MUST NOT）。`gc` は summary として少なくとも `scanned_dirs`, `candidate_count`, `deleted`, `skipped`, `out_of_scope`, `failed`, `freed_bytes` を返さなければならない（MUST）。`gc` は `jobs` field をレスポンスに含めてはならない（MUST NOT）。

#### Scenario: gc returns deleted count only after the directory is gone

**Given**: retention 条件を満たす terminal job directory が存在する
**When**: `agent-exec gc --older-than 7d` を実行する
**Then**: レスポンスの `deleted` は command 完了時点で存在しないことを確認できた directory だけを集計する
**And**: command 完了時点でも directory が存在する場合は `deleted` に集計しない
**And**: レスポンスに `jobs` field は含まれない

#### Scenario: gc dry-run reports summary without deleting

**Given**: root 配下に terminal, running, created, unknown job directory が混在している
**When**: `agent-exec gc --dry-run --older-than 7d` を実行する
**Then**: レスポンスは `scanned_dirs`, `candidate_count`, `deleted`, `skipped`, `out_of_scope`, `failed`, `freed_bytes` を含む
**And**: `deleted` は `0` である
**And**: deletion candidate の job directory は command 完了時点でまだ存在する
**And**: レスポンスに `jobs` field は含まれない

#### Scenario: gc applies max job count cleanup to terminal jobs only

**Given**: root 配下に retention window 内外の terminal jobs と running jobs が存在する
**When**: `agent-exec gc --max-jobs 10 --dry-run` を実行する
**Then**: newest 10 件を超える terminal jobs だけが count pressure の削除候補として `candidate_count` に集計される
**And**: running jobs は件数圧力による削除候補にならない
**And**: レスポンスに `jobs` field は含まれない

#### Scenario: gc applies max byte cleanup to terminal jobs only

**Given**: root 全体の bytes が `--max-bytes` を超えている
**When**: `agent-exec gc --max-bytes <BYTES> --dry-run` を実行する
**Then**: 古い terminal jobs が byte pressure の削除候補として `candidate_count` と `freed_bytes` に集計される
**And**: running jobs と created jobs は byte pressure の削除候補にならない
**And**: レスポンスに `jobs` field は含まれない
