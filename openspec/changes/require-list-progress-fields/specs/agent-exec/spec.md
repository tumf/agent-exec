## MODIFIED Requirements

### Requirement: list の JSON ペイロード

`list` は `{"schema_version","ok","type":"list","jobs":[...]}` を返さなければならない（MUST）。`jobs[]` の各エントリは `job_id`・`state`・`created_at` を必ず含まなければならない（MUST）。

state.json が読める場合、各エントリは `updated_at` を必ず含めなければならない（MUST）。ジョブが終端状態（succeeded / failed / killed / timeout）の場合、`finished_at` と `exit_code` を必ず含めなければならない（MUST）。state.json がレース条件で未作成・破損している場合に限り、これらは省略してよい（MAY）。

#### Scenario: list includes progress for running jobs

**Given**: a running job whose state.json is readable
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`
**And**: `finished_at` and `exit_code` are absent

#### Scenario: list includes terminal fields for finished jobs

**Given**: a finished job
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`, `finished_at`, and `exit_code`
