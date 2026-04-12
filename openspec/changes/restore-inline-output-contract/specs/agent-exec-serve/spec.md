## MODIFIED Requirements

### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須）と任意の `cwd`・`env`・`timeout_ms` を受け取り、CLI の `run` と同等の既定待機・inline output 契約でジョブを起動して返さなければならない（MUST）。`wait` を受け付けてはならない（MUST NOT）。

#### Scenario: POST /exec は CLI run と同じ output fields を返す

Given `agent-exec serve` が起動している
When `POST /exec` に `{"command": ["echo", "hi"]}` を送る
Then HTTP 200 かつ `job_id`, `stdout`, `stdout_range`, `stdout_total_bytes` を含む JSON が返る
And 削除済み snapshot-era field 名は含まれない

### Requirement: GET /tail/:id によるログ末尾取得

`GET /tail/:id` は CLI の `tail` と同等の応答を返さなければならない（MUST）。末尾本文は `stdout` / `stderr` と range 情報で表現しなければならない（MUST）。

#### Scenario: GET /tail は range 付き tail shape を返す

Given 完了したジョブが存在する
When `GET /tail/<job_id>` をリクエストする
Then HTTP 200 かつ `stdout`, `stdout_range`, `stdout_total_bytes` フィールドを含む JSON が返る
And `stdout_tail` は含まれない
