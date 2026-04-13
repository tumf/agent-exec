## MODIFIED Requirements

### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須、string 配列）と任意の `cwd`・`env`・`timeout`（秒）・`wait`（bool、既定 true）・`until`（秒、既定 10）・`max_bytes`（u64、既定 65536）を受け取り、CLI `run` と同じ inline 観測契約でジョブを起動して返さなければならない（MUST）。旧 `timeout_ms` は受け付けてはならない（MUST NOT）。`wait`/`until`/`max_bytes` はクライアントが上書きできなければならない（MUST）。

`POST /exec` のレスポンスは CLI `run` と同じ inline output field（`stdout`/`stderr` と range/total bytes、および終端フィールド）を返さなければならない（MUST）。新規 job の `job_id` は hash-like 小文字 hex ID でなければならない（MUST）。

#### Scenario: POST /exec accepts until override

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command":["sh","-c","exit 7"],"until":1}` を送る
**Then**: HTTP 200 かつ `exit_code=7` を含む JSON が約 1 秒で返る

#### Scenario: POST /exec accepts wait=false

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command":["sleep","60"],"wait":false}` を送る
**Then**: HTTP 200 が即座に返る
**And**: `stdout` は空または省略される

#### Scenario: POST /exec rejects legacy timeout_ms

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command":["echo","hi"],"timeout_ms":1000}` を送る
**Then**: HTTP 400 が返る
