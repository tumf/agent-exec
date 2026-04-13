## MODIFIED Requirements

### Requirement: HTTP エンドポイントの job_id 解決は CLI と共通

`GET /status/:id`・`GET /tail/:id`・`GET /wait/:id`・`POST /kill/:id` は `:id` として完全な `job_id` または一意な先頭 prefix を受理しなければならない（MUST）。解決規則（hex/ULID 形式混在時の挙動、prefix 最小長、衝突時の `ambiguous_job_id` エラー形）は canonical `agent-exec` spec の prefix 解決 Requirement に従わなければならない（MUST）。重複定義は置かない（MUST NOT）。

#### Scenario: HTTP follows canonical prefix rules

**Given**: a hash-like job exists and its 7-character prefix is unique
**When**: `GET /status/<prefix>` is requested
**Then**: HTTP 200 with the job's state is returned
**And**: the resolution behavior is identical to `agent-exec status <prefix>`

#### Scenario: HTTP ambiguous prefix returns canonical error code

**Given**: 2 jobs share a prefix
**When**: `GET /status/<shared-prefix>` is requested
**Then**: HTTP 400 with `error.code="ambiguous_job_id"` is returned
