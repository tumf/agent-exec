## ADDED Requirements

### Requirement: エラーレスポンスの構造化 details

エラーレスポンスの `error` オブジェクトは `code`・`message`・`retryable` に加え、任意の構造化補足情報を `details`（JSON object）として含めてよい（MAY）。`details` は安定したキー集合を持つ error code ごとにスキーマを規定する（MUST）。

`error.code = "ambiguous_job_id"` の場合、`details` は以下を必ず含めなければならない（MUST）:
- `candidates`: 衝突した完全な `job_id` の配列。最大 20 件まで。
- `truncated`: 候補が 20 件を超えたときに `true`、そうでなければ `false`。

#### Scenario: ambiguous_job_id returns structured candidates

**Given**: 2 jobs share a common prefix
**When**: `agent-exec status <shared-prefix>` is executed
**Then**: the response includes `error.code="ambiguous_job_id"`
**And**: `error.details.candidates` is an array of length ≥ 2
**And**: `error.details.truncated` is `false`

#### Scenario: ambiguous_job_id truncates large candidate sets

**Given**: 25 jobs share a common prefix
**When**: `agent-exec status <shared-prefix>` is executed
**Then**: `error.details.candidates` contains 20 entries
**And**: `error.details.truncated` is `true`
