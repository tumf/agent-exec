## MODIFIED Requirements

### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須、string 配列）と任意の `cwd`・`env`・`abandon_job_after`（秒）・`acknowledge_result_loss`（bool）・`wait`（bool、既定 true）・`until`（秒、既定 10）・`max_bytes`（u64、既定 65536）を受け取り、CLI `run` と同じ既定待機・inline 観測契約でジョブを起動して `RunData` を返さなければならない（MUST）。旧 `timeout` / `timeout_ms` は受け付けてはならない（MUST NOT）。`wait`/`until`/`max_bytes` はクライアントが上書きできなければならない（MUST）。

`POST /exec` のレスポンスは CLI `run` と同じ inline output field（`stdout`/`stderr` と range/total bytes、および終端フィールド）を返さなければならない（MUST）。新規 job の `job_id` は hash-like 小文字 hex ID でなければならない（MUST）。

`abandon_job_after` のschema descriptionは、この制御がjobを諦めて未完了成果を永久に失う可能性があり、待機だけを止める場合は `until` を使う旨の警告から始めなければならない（MUST）。non-null `abandon_job_after` は `acknowledge_result_loss=true` を要求し、欠落またはfalseならHTTP 400でjob作成前に拒否しなければならない（MUST）。legacy `timeout` / `timeout_ms` のHTTP 400 errorは `abandon_job_after` と `until` の両方を示さなければならない（MUST）。`until` expiryはmanaged jobへsignalしてはならない（MUST NOT）。

#### Scenario: ジョブ起動成功

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command": ["echo", "hi"]}` を送る
**Then**: HTTP 200 かつ `job_id` を含む JSON が返る

#### Scenario: command フィールド欠落

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{}` を送る
**Then**: HTTP 400 かつ `ok=false` の JSON が返る

#### Scenario: POST /exec は CLI run と同じ output fields を返す

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command": ["echo", "hi"]}` を送る
**Then**: HTTP 200 かつ `job_id`, `stdout`, `stdout_range`, `stdout_total_bytes` を含む JSON が返る
**And**: 削除済み snapshot-era field 名は含まれない

#### Scenario: POST /exec returns a hash-like job ID

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command": ["echo", "hi"]}` を送る
**Then**: HTTP 200 かつ `job_id` を含む JSON が返る
**And**: `job_id` は `[0-9a-f]` のみで構成される固定長文字列である

#### Scenario: POST /exec accepts until override

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command":["sh","-c","exit 7"],"until":1}` を送る
**Then**: HTTP 200 かつ `exit_code=7` を含む JSON が約 1 秒で返る

#### Scenario: POST /exec accepts wait=false

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command":["sleep","60"],"wait":false}` を送る
**Then**: HTTP 200 が即座に返る
**And**: `stdout` は空または省略される

#### Scenario: POST /exec rejects legacy timeout fields

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command":["echo","hi"],"timeout":1}` を送る
**Then**: HTTP 400 が返る

#### Scenario: POST /exec rejects unacknowledged abandonment

**Given**: `agent-exec serve` is running
**When**: `/exec` receives `abandon_job_after=1` without `acknowledge_result_loss=true`
**Then**: the response is HTTP 400 with the result-loss warning
**And**: no job is created

#### Scenario: POST /exec accepts acknowledged abandonment

**Given**: `agent-exec serve` is running
**When**: `/exec` receives a command exceeding `abandon_job_after=1` with `acknowledge_result_loss=true`
**Then**: the managed workload is terminated by the canonical escalation behavior
**And**: result projections identify actual abandonment

#### Scenario: POST /exec until expiry leaves the job running

**Given**: `/exec` receives a workload that runs longer than `until=1`
**When**: inline observation ends
**Then**: the response reports a non-terminal job
**And**: the service has not signaled the workload
