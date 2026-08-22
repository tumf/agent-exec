## MODIFIED Requirements

### Requirement: timeout と kill-after

`--abandon-job-after` が `--acknowledge-result-loss` と共に指定された場合、期限到達時に終了シグナルを送信し、`--kill-after` 経過後も生存している場合は強制終了しなければならない（MUST）。既定は無制限でなければならない（MUST）。Help は、この制御がjobを諦めて未完了成果を永久に失う可能性があり、待機だけを止める場合は `until` を使う旨の警告から始めなければならない（MUST）。acknowledgementなし、またはhidden migration trap `--timeout` の指定はjob作成前に拒否し、エラーは `--abandon-job-after` と `--until` の両方を示さなければならない（MUST）。private `_supervise --timeout` handoffは公開surfaceではなく、内部整合を保ったまま維持しなければならない（MUST）。

実際に abandonment が workload を終了させた場合、終端 state 値自体は変更してはならず（MUST NOT）、追加の provenance field `abandoned_by="abandon_job_after"` と `result_loss=true` を persisted state とその status / list / completion-event projection に出力しなければならない（MUST）。設定した期限が発火しなかった場合、およびそれらの field を持たない既存 state を読む場合は、両 field を出力してはならない（MUST NOT）。

Persisted job definition は、1 回の migration release の間、等価な `abandon_job_after_ms` と `timeout_ms` を dual-write しなければならない（MUST）。Reader は legacy-only、new-only、equal dual のいずれも受け付けなければならず（MUST）、両者が不一致の場合は start / restart の前に fail closed し、エラーに job ID と両 field 名・両値を含めなければならない（MUST）。

#### Scenario: abandon-job-after の強制終了
Given `agent-exec run --abandon-job-after 1 --acknowledge-result-loss --kill-after 1s -- <cmd>` を実行する
When 2 秒経過する
Then 対象プロセスは終了している

#### Scenario: unacknowledged abandonment is rejected before job creation

**Given**: a user executes `agent-exec run --abandon-job-after 1 -- sleep 60`
**When**: CLI arguments are validated
**Then**: the command fails with a result-loss warning
**And**: no job is created

#### Scenario: legacy CLI spelling returns migration guidance

**Given**: a user executes `agent-exec run --timeout 1 -- sleep 60`
**When**: CLI arguments are validated
**Then**: the command fails before job creation
**And**: the error names `--abandon-job-after`, `--acknowledge-result-loss`, and `--until`

#### Scenario: until expiry preserves the workload

**Given**: a managed job runs longer than one second
**When**: observation returns after `until=1`
**Then**: the response is non-terminal
**And**: the managed job receives no termination signal

#### Scenario: actual abandonment is durably identifiable

**Given**: an acknowledged `abandon-job-after` limit actually terminates a workload
**When**: state, status, list, or completion output is read
**Then**: the existing terminal state value is unchanged
**And**: `abandoned_by="abandon_job_after"` and `result_loss=true` identify the abandonment

#### Scenario: a limit that never fires leaves no abandonment markers

**Given**: a job configured with an acknowledged `abandon-job-after` limit finishes before the limit
**When**: state, status, list, or completion output is read
**Then**: `abandoned_by` and `result_loss` are absent
**And**: historical records written without those fields remain readable without synthesized provenance

#### Scenario: migration metadata preserves downgrade behavior

**Given**: the migration release creates a persisted job definition with a nonzero abandonment limit
**When**: metadata is written
**Then**: equal `abandon_job_after_ms` and `timeout_ms` values are present
**And**: an older binary preserves the same limit when starting or restarting the job

#### Scenario: unequal persisted runtime fields fail closed

**Given**: persisted metadata contains unequal `timeout_ms` and `abandon_job_after_ms` values
**When**: the definition is loaded
**Then**: loading fails before launch
**And**: the error names the job ID, both fields, and both values

### Requirement: 人間向け runtime 制御時間は秒単位である

`run`、`create`、および同じ人間向け CLI surface を共有する関連サブコマンドが受け付ける runtime 制御時間オプション (`--abandon-job-after`, `--kill-after`, `--progress-every`) は秒単位で解釈しなければならない（MUST）。内部実装でミリ秒へ変換してもよいが、clap help、README、skills、統合テストは秒単位を正規表現として扱わなければならない（MUST）。

#### Scenario: run abandon-job-after is interpreted in seconds

**Given**: a user executes `agent-exec run --abandon-job-after 30 --acknowledge-result-loss -- sh -c "sleep 60"`
**When**: the runtime limit is applied
**Then**: `30` is interpreted as 30 seconds
**And**: it is not interpreted as 30 milliseconds

#### Scenario: create persists second-based runtime controls

**Given**: a user executes `agent-exec create --abandon-job-after 30 --acknowledge-result-loss --kill-after 5 --progress-every 1 -- sh -c "sleep 60"`
**When**: the persisted job definition is created
**Then**: the human-facing contract for those values is seconds
