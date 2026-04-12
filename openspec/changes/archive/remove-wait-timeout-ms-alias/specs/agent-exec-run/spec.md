## MODIFIED Requirements

### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30,000ms までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <ms>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

`--timeout-ms` は `wait` サブコマンドの有効なオプションとして受け付けてはならない（MUST NOT）。待機期限指定は `--until` に統一しなければならない（MUST）。

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

#### Scenario: wait rejects removed timeout-ms spelling

Given a user executes `agent-exec wait --timeout-ms 100 <job_id>`
When clap validates arguments
Then the command fails with usage error
And stdout is empty
