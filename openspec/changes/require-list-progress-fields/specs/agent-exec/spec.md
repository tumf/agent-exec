## MODIFIED Requirements

### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `short_job_id`, `state`, `started_at` を含まなければならない（MUST）。

state.json が読める場合、各エントリは `updated_at` を必ず含めなければならない（MUST）。ジョブが終端状態（succeeded / failed / killed / timeout）の場合、`finished_at` と `exit_code` を必ず含めなければならない（MUST）。state.json がレース条件で未作成・破損している場合に限り、これらは省略してよい（MAY）。

#### Scenario: list が必須フィールドを返す

Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `short_job_id`, `state`, `started_at` を含む

#### Scenario: list includes progress for running jobs

**Given**: a running job whose state.json is readable
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`
**And**: `finished_at` and `exit_code` are absent

#### Scenario: list includes terminal fields for finished jobs

**Given**: a finished job
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`, `finished_at`, and `exit_code`
