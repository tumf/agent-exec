## ADDED Requirements

### Requirement: 人間向け runtime 制御時間は秒単位である

`run`、`create`、および同じ人間向け CLI surface を共有する関連サブコマンドが受け付ける runtime 制御時間オプション (`--timeout`, `--kill-after`, `--progress-every`) は秒単位で解釈しなければならない（MUST）。内部実装でミリ秒へ変換してもよいが、clap help、README、skills、統合テストは秒単位を正規表現として扱わなければならない（MUST）。

#### Scenario: run timeout is interpreted in seconds

**Given**: a user executes `agent-exec run --timeout 30 -- sh -c "sleep 60"`
**When**: the runtime limit is applied
**Then**: `30` is interpreted as 30 seconds
**And**: it is not interpreted as 30 milliseconds

#### Scenario: create persists second-based runtime controls

**Given**: a user executes `agent-exec create --timeout 30 --kill-after 5 --progress-every 1 -- sh -c "sleep 60"`
**When**: the persisted job definition is created
**Then**: the human-facing contract for those values is seconds

### Requirement: 削除済み snapshot-era guidance は正規 surface に残さない

削除済みの `snapshot-after` およびそれに依存する旧 guidance は、現行 CLI の正規 help、README、skills、統合テストに残してはならない（MUST NOT）。現行の `run` は即時返却し、観測責務は `wait` / `tail` / `status` に分離されていることを正規 docs が示さなければならない（MUST）。

#### Scenario: removed snapshot option is rejected

**Given**: a user executes `agent-exec run --snapshot-after 10 -- echo hi`
**When**: CLI arguments are validated
**Then**: the command fails with usage error

#### Scenario: skills no longer teach snapshot-after

**Given**: a user reads `skills/agent-exec/**`
**When**: they look for current run examples
**Then**: the live examples do not require `--snapshot-after 0` to explain immediate return

## MODIFIED Requirements

### Requirement: run の既定スナップショットと出力含有

`run` は返却前に観測用 snapshot を生成するための追加待機を行ってはならない（MUST NOT）。`run` の主責務は job 起動と `job_id` / 初期 state / ログパスの返却であり、完了待機と出力観測は `wait` / `tail` / `status` に分離しなければならない（MUST）。

#### Scenario: default run returns immediately without snapshot wait

**Given**: `agent-exec run -- sh -c "sleep 1; echo hi"` is executed
**When**: the JSON response is returned
**Then**: `job_id` is present
**And**: `snapshot` is absent
**And**: `final_snapshot` is absent

### Requirement: run は削除済み snapshot オプションを拒否する

`run` は `snapshot-after`、`tail-lines`、`max-bytes`、および削除済み観測系フラグを受け付けてはならない（MUST NOT）。

#### Scenario: run rejects removed snapshot-after option

**Given**: `agent-exec run --snapshot-after 10 -- echo hi` is executed
**When**: CLI arguments are validated
**Then**: the command fails with usage error
