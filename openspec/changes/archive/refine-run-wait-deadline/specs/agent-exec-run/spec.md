## MODIFIED Requirements

### Requirement: run の同期待機オプション

`run` は `--wait` が指定された場合、既定では最大 30,000ms までジョブの状態変化を待機しなければならない（MUST）。待機上限は `--until <ms>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態 (`exited|killed|failed`) になるまで無制限に待機しなければならない（MUST）。

`--until` と `--forever` は `--wait` と組み合わせる観測用 option であり、`--timeout` が表すジョブ実行時間の timeout とは別概念として扱わなければならない（MUST）。`--until` と `--forever` は単独使用を許可してはならず（MUST NOT）、互いに同時指定も許可してはならない（MUST NOT）。

`--wait` 指定時、`run` は待機上限に達しただけではジョブを終了させてはならない（MUST NOT）。終端状態まで到達した場合の `run` JSON は `exit_code`（存在する場合）と `finished_at` と `final_snapshot` を含めなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合、`run` JSON は非終端の `state` を返し、`exit_code` / `finished_at` / `final_snapshot` を含めてはならない（MUST NOT）。`waited_ms` は実際に待機した時間を示さなければならない（MUST）。

#### Scenario: --wait uses the default 30 second deadline

Given `agent-exec run --wait -- sh -c "sleep 1; echo hi"` is executed
When the command finishes within the default wait deadline
Then the response state is `exited`
And `final_snapshot.stdout_tail` contains `hi`
And `finished_at` is present

#### Scenario: --wait --until returns while the job keeps running

Given `agent-exec run --wait --until 100 -- sh -c "sleep 2; echo hi"` is executed
When the wait deadline is reached before the job exits
Then the response state is `created` or `running`
And `finished_at` is absent
And `final_snapshot` is absent
And the job continues running after the `run` command returns

#### Scenario: --wait --forever preserves unbounded waiting

Given `agent-exec run --wait --forever -- sh -c "sleep 1; echo hi"` is executed
When the job eventually exits
Then the response state is `exited`
And `final_snapshot.stdout_tail` contains `hi`

#### Scenario: wait-deadline flags require --wait

Given a user executes `agent-exec run --until 100 -- sh -c "echo hi"`
When clap validates arguments
Then the command fails with usage error

And given a user executes `agent-exec run --forever -- sh -c "echo hi"`
When clap validates arguments
Then the command fails with usage error

#### Scenario: --until and --forever are mutually exclusive

Given a user executes `agent-exec run --wait --until 100 --forever -- sh -c "echo hi"`
When clap validates arguments
Then the command fails with usage error

### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30,000ms までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <ms>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

既存の `--timeout-ms` オプションは `--until` に置換する（MUST）。

#### Scenario: wait uses the default 30 second deadline

Given a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
When `agent-exec wait <job_id>` is executed
Then the wait returns within approximately 30 seconds
And if the job finished within the deadline, the response state is terminal

#### Scenario: wait --until returns while the job keeps running

Given a running job created by `agent-exec run -- sh -c "sleep 10"`
When `agent-exec wait --until 100 <job_id>` is executed
Then the response state is `created` or `running`
And `exit_code` is absent

#### Scenario: wait --forever preserves unbounded waiting

Given a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
When `agent-exec wait --forever <job_id>` is executed
Then the response state is terminal after the job exits

#### Scenario: wait --until and --forever are mutually exclusive

Given a user executes `agent-exec wait --until 100 --forever <job_id>`
When clap validates arguments
Then the command fails with usage error
