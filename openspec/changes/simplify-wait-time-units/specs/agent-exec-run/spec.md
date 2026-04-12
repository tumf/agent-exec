## ADDED Requirements

### Requirement: 人間向け待機期限オプションは秒単位である

`run --wait` と `wait` が受け付ける人間向け待機期限オプションは秒単位で解釈しなければならない（MUST）。既定の待機期限は 30 秒でなければならない（MUST）。内部実装でミリ秒や `Duration` に変換してもよいが、CLI 契約・ヘルプ・ドキュメント・統合テストは秒単位を正規表現として扱わなければならない（MUST）。

#### Scenario: wait uses second-based until

**Given**: a running job created by `agent-exec run -- sh -c "sleep 10"`
**When**: `agent-exec wait --until 30 <job_id>` is executed
**Then**: the command interprets `30` as 30 seconds
**And**: the wait deadline is not interpreted as 30 milliseconds

#### Scenario: run --wait uses second-based until

**Given**: a user executes `agent-exec run --wait --until 30 -- sh -c "sleep 10"`
**When**: clap accepts the arguments and the command waits for terminal state
**Then**: the wait deadline is interpreted as 30 seconds

### Requirement: 人間向けポーリング間隔オプションは秒単位である

`wait` および `run --wait` の人間向けポーリング間隔オプションは秒単位で表現しなければならない（MUST）。ポーリングは観測用の近似間隔であり、ミリ秒精度の厳密なチェック時刻を保証してはならない（MUST NOT）。

#### Scenario: wait exposes second-based poll option

**Given**: a user inspects `agent-exec wait --help`
**When**: the polling option is shown
**Then**: the canonical polling option is documented in seconds
**And**: the help text does not imply millisecond-accurate checking

#### Scenario: run --wait exposes second-based poll option

**Given**: a user inspects `agent-exec run --help`
**When**: the wait polling option is shown
**Then**: the canonical polling option is documented in seconds

## MODIFIED Requirements

### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30 秒までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <seconds>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

待機期限指定は秒単位の `--until` に統一しなければならない（MUST）。ミリ秒前提の旧語彙や旧解釈を残す場合は、互換または拒否の挙動を明示的に定義しなければならない（MUST）。

#### Scenario: wait uses the default 30 second deadline

**Given**: a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
**When**: `agent-exec wait <job_id>` is executed
**Then**: the wait returns within approximately 30 seconds
**And**: if the job finished within the deadline, the response state is terminal

#### Scenario: wait --until returns while the job keeps running

**Given**: a running job created by `agent-exec run -- sh -c "sleep 10"`
**When**: `agent-exec wait --until 1 <job_id>` is executed
**Then**: the response state is `created` or `running`
**And**: `exit_code` is absent

#### Scenario: wait --forever preserves unbounded waiting

**Given**: a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
**When**: `agent-exec wait --forever <job_id>` is executed
**Then**: the response state is terminal after the job exits

### Requirement: run --wait の待機期限オプション

`run` は `--wait` が指定された場合、既定では最大 30 秒までジョブの状態変化を待機しなければならない（MUST）。待機上限は `--until <seconds>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態 (`exited|killed|failed`) になるまで無制限に待機しなければならない（MUST）。

`--until` と `--forever` は `--wait` と組み合わせる観測用オプションであり、同時指定してはならない（MUST NOT）。`--wait` なしで `--until` / `--forever` を受け付けてはならない（MUST）。待機期限指定は秒単位で統一しなければならない（MUST）。

#### Scenario: --wait --until uses second-based deadline

**Given**: `agent-exec run --wait --until 1 --snapshot-after 0 -- sleep 60` is executed
**When**: the wait deadline is reached before the job exits
**Then**: the response state is `created` or `running`
**And**: the job continues running after the `run` command returns

#### Scenario: wait-deadline flags require --wait

**Given**: a user executes `agent-exec run --until 1 -- sh -c "echo hi"`
**When**: clap validates arguments
**Then**: the command fails with usage error
