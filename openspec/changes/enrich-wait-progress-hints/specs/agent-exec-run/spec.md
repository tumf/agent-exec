## MODIFIED Requirements

### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30 秒までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <seconds>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

`wait` のレスポンスは、state.json が読める場合、`stdout_total_bytes`・`stderr_total_bytes`・`updated_at` を必ず含めなければならない（MUST）。これは満期到達時・終端到達時の両方に適用される（MUST）。state.json がレース条件で未作成の場合はこれらを省略してよい（MAY）。

待機期限指定は秒単位の `--until` に統一しなければならない（MUST）。`--timeout-ms` は有効なオプションとして受け付けてはならない（MUST NOT）。

`wait` のポーリング間隔は秒単位の `--poll <seconds>` で指定できなければならない（MUST）。この間隔は観測用の近似値であり、ミリ秒精度の厳密なチェック時刻を保証してはならない（MUST NOT）。

#### Scenario: wait timeout returns progress hints

**Given**: a running job created by `agent-exec run -- sh -c "sleep 10"`
**When**: `agent-exec wait --until 1 <job_id>` is executed
**Then**: the response includes `stdout_total_bytes`, `stderr_total_bytes`, and `updated_at`
**And**: `exit_code` is absent

#### Scenario: wait terminal response also returns progress hints

**Given**: a finished job
**When**: `agent-exec wait <job_id>` returns its terminal response
**Then**: the response includes `stdout_total_bytes`, `stderr_total_bytes`, and `updated_at`
**And**: `exit_code` is present
