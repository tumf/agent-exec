## MODIFIED Requirements

### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須）と任意の `cwd`・`env`・`timeout_ms` を受け取り、CLI の `run` と同等の既定待機・inline output 契約でジョブを起動して返さなければならない（MUST）。新規 job の `job_id` は hash-like 小文字 hex ID でなければならない（MUST）。`wait` を受け付けてはならない（MUST NOT）。

#### Scenario: POST /exec returns a hash-like job ID

Given `agent-exec serve` が起動している
When `POST /exec` に `{"command": ["echo", "hi"]}` を送る
Then HTTP 200 かつ `job_id` を含む JSON が返る
And `job_id` は `[0-9a-f]` のみで構成される固定長文字列である

### Requirement: GET /status/:id によるジョブ状態取得

`GET /status/:id` は `status` サブコマンドと同等の応答を HTTP 200 で返さなければならない（MUST）。`:id` は完全な `job_id` だけでなく一意な先頭 prefix も受け付けなければならない（MUST）。job_id が存在しない場合は HTTP 404 を返さなければならない（MUST）。prefix が複数 job に一致する場合は HTTP 400 と `error.code="ambiguous_job_id"` を返さなければならない（MUST）。

#### Scenario: status resolves a unique prefix

Given `POST /exec` で作成された hash-like job が存在する
And その先頭 prefix が一意である
When `GET /status/<prefix>` をリクエストする
Then HTTP 200 かつ対応する job の状態を含む JSON が返る

#### Scenario: status rejects an ambiguous prefix

Given 同じ先頭 prefix を共有する 2 件の job が存在する
When `GET /status/<shared-prefix>` をリクエストする
Then HTTP 400 かつ `error.code="ambiguous_job_id"` を含む JSON が返る
