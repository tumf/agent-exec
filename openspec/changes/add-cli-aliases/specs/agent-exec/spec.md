## ADDED Requirements

### Requirement: common CLI aliases for job inspection and deletion

`agent-exec` は、既存の job inspection / deletion 操作に対して短い CLI alias を提供しなければならない（MUST）。`ps` は `list --state running` の shorthand として振る舞い、running job だけを返さなければならない（MUST）。`rm` は `delete` の alias として振る舞い、明示 job delete と bulk delete の既存契約を変えてはならない（MUST NOT）。

#### Scenario: ps lists only running jobs

**Given**: 実行中ジョブと終了済みジョブが存在する
**When**: `agent-exec ps` を実行する
**Then**: `jobs` は `state=running` のジョブのみを含む
**And**: `agent-exec list --state running` と同じ集合を返す

#### Scenario: ps preserves list filtering knobs except state

**Given**: cwd や tag が異なる複数の running job が存在する
**When**: `agent-exec ps --all` または `agent-exec ps --cwd <PATH>` または `agent-exec ps --tag <PATTERN>` を実行する
**Then**: それぞれ `agent-exec list --state running` に同じ filter option を付けた場合と同じ絞り込み結果を返す

#### Scenario: rm behaves like delete

**Given**: delete 対象となる terminal job が存在する
**When**: `agent-exec rm <job_id>` を実行する
**Then**: `agent-exec delete <job_id>` と同じ削除契約で処理される

#### Scenario: rm preserves bulk delete behavior

**Given**: bulk delete 対象の terminal jobs が存在する
**When**: `agent-exec rm --all` または `agent-exec rm --dry-run --all` を実行する
**Then**: `agent-exec delete --all` または `agent-exec delete --dry-run --all` と同じ bulk delete 契約で処理される
