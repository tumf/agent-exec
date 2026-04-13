## ADDED Requirements

### Requirement: kill は既定で post-signal 観測を返す

`agent-exec kill <job_id>` は signal 送信後に最大 3 秒までジョブの状態遷移を観測し、観測した `state`・`exit_code`（終端到達時のみ）・`terminated_signal`（signal 終了時のみ）・`observed_within_ms` を含むレスポンスを返さなければならない（MUST）。`--no-wait` が指定された場合は signal 送信結果のみの従来 shape（`job_id` / `signal`）を返さなければならない（MUST）。

`POST /kill/:id` は同じ観測契約に従わなければならない（MUST）。クエリ `no_wait=true` で opt-out 可能でなければならない（MUST）。

#### Scenario: kill observes termination within budget

**Given**: a running job
**When**: `agent-exec kill <job_id>` is executed
**Then**: the response includes `state=killed` and `exit_code` or `terminated_signal`
**And**: `observed_within_ms` is present

#### Scenario: kill --no-wait preserves legacy shape

**Given**: a running job
**When**: `agent-exec kill --no-wait <job_id>` is executed
**Then**: the response contains `job_id` and `signal` only
**And**: `state` is absent
