## MODIFIED Requirements

### Requirement: delete と gc の削除結果可観測性

`delete` と `gc` は job directory を削除したと報告する場合、`remove_dir_all` が成功しただけでは不十分であり、コマンド完了時点で対象 job directory が存在しないことを確認してから `action="deleted"` を返さなければならない（MUST）。削除呼び出し後も対象 path が存在する場合は、削除成功として扱ってはならない（MUST NOT）。

`delete --all` は bulk delete の評価に使った effective cwd scope をレスポンスに含めなければならない（MUST）。`gc` と `delete` のレスポンスは、利用者が少なくとも「削除成功」「対象内だが削除されなかった」「対象外または条件不一致」を識別できるだけの action/reason または集計情報を含まなければならない（MUST）。

#### Scenario: delete returns deleted only after the directory is gone

**Given**: `delete <job_id>` の対象となる terminal job directory が存在する
**When**: `agent-exec delete <job_id>` を実行する
**Then**: レスポンスでその job に `action="deleted"` を返すのは command 完了時点で当該 directory が存在しない場合だけである
**And**: command 完了時点でも directory が存在する場合は `action="deleted"` を返さない

#### Scenario: gc returns deleted only after the directory is gone

**Given**: retention 条件を満たす terminal job directory が存在する
**When**: `agent-exec gc --older-than 7d` を実行する
**Then**: レスポンスでその job に `action="deleted"` を返すのは command 完了時点で当該 directory が存在しない場合だけである
**And**: command 完了時点でも directory が存在する場合は `action="deleted"` を返さない

#### Scenario: delete all exposes effective cwd scope

**Given**: current_dir が `A` であり、`meta.json.cwd == A` の terminal job と `meta.json.cwd == B` の terminal job が混在している
**When**: `agent-exec delete --all` を実行する
**Then**: レスポンスは bulk delete 評価に使った effective cwd scope を含む
**And**: 利用者はそのレスポンスから `A` scope に対する削除評価であることを確認できる

#### Scenario: delete and gc distinguish skipped vs out-of-scope behavior

**Given**: `running`、`too_recent`、cwd mismatch、削除成功の job が同時に存在する
**When**: `agent-exec delete --all` または `agent-exec gc` を実行する
**Then**: レスポンスから各 job または集計が「削除成功」「対象内だが削除されなかった」「対象外または条件不一致」のどれに該当するかを識別できる
