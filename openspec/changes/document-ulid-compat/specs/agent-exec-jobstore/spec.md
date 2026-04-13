## ADDED Requirements

### Requirement: ULID/hex 併存時の読み取り互換

新規に生成する `job_id` は 32 文字小文字 hex でなければならない（MUST）。ただし過去に ULID 形式（Crockford base32 26 文字）で生成された既存ディレクトリは読み取り可能でなければならない（MUST）。

`JobDir::open` は指定された prefix を文字列として解釈し、`0-9a-zA-Z` の範囲で一致するすべての job ディレクトリを候補とする（MUST）。hex と ULID のどちらに一致するかで暗黙の優先順位を付けてはならない（MUST NOT）。prefix が 2 件以上にマッチする場合は `ambiguous_job_id` エラーを返し、候補にはマッチした hex / ULID を両方含めなければならない（MUST）。

#### Scenario: ULID-format legacy job is readable

**Given**: a legacy job directory whose `job_id` is 26-char Crockford base32
**When**: `agent-exec status <legacy-id>` is executed
**Then**: HTTP 200 / exit 0 with the job's state is returned

#### Scenario: prefix matching both hex and ULID returns ambiguous

**Given**: a hex job `01abc...` and a ULID job `01ABC...` (case-insensitive overlap)
**When**: `agent-exec status 01` is executed
**Then**: `error.code="ambiguous_job_id"` is returned
**And**: `error.details.candidates` contains both job ids
