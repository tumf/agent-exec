## MODIFIED Requirements

### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `short_job_id`, `state`, `started_at`, `command` を含まなければならない（MUST）。`command` は当該 job の `meta.json.command` と同じ string array であり、argv の順序を保持しなければならない（MUST）。

state.json が読める場合、各エントリは `updated_at` を必ず含めなければならない（MUST）。ジョブが終端状態（succeeded / failed / killed / timeout）の場合、`finished_at` と `exit_code` を必ず含めなければならない（MUST）。state.json がレース条件で未作成・破損している場合に限り、これらは省略してよい（MAY）。

#### Scenario: list が必須フィールドを返す

Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `short_job_id`, `state`, `started_at`, `command` を含む

#### Scenario: list includes the persisted command

**Given**: a job was created with command arguments `["sh", "-c", "echo hi"]`
**When**: `agent-exec list` is executed and returns that job
**Then**: the job entry includes `command=["sh", "-c", "echo hi"]`
**And**: the command argument order is unchanged

#### Scenario: list includes progress for running jobs

**Given**: a running job whose state.json is readable
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`
**And**: `finished_at` and `exit_code` are absent

#### Scenario: list includes terminal fields for finished jobs

**Given**: a finished job
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`, `finished_at`, and `exit_code`
