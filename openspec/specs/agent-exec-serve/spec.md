## Requirements

### Requirement: serve サブコマンドの提供

`agent-exec serve` は HTTP サーバを起動し、ジョブ操作を REST API として公開しなければならない（MUST）。

#### Scenario: serve の起動

**Given**: `agent-exec serve` を実行する
**When**: サーバが起動する
**Then**: デフォルトで `127.0.0.1:19263` で HTTP リクエストを受け付ける（外部ネットワークには公開されない）

### Requirement: バインドアドレスの設定

`--bind <ADDR>` オプションでリスンアドレスを上書きできなければならない（MUST）。`--port <PORT>` オプションでポートのみ変更できなければならない（MUST）。デフォルトのポートは `19263` でなければならない（MUST）。

#### Scenario: --bind でアドレスを限定する

**Given**: `agent-exec serve --bind 127.0.0.1:19999` を実行する
**When**: サーバが起動する
**Then**: `127.0.0.1:19999` でリクエストを受け付ける

### Requirement: GET /health

`GET /health` はサーバが正常稼働中に `{"ok":true,"type":"health"}` を含む JSON を HTTP 200 で返さなければならない（MUST）。

#### Scenario: ヘルスチェック

**Given**: `agent-exec serve` が起動している
**When**: `GET /health` をリクエストする
**Then**: HTTP 200 かつ `{"ok":true}` を含む JSON が返る

### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須）と任意の `cwd`・`env`・`timeout`（秒、CLI `--timeout` と同じ秒単位契約） を受け取り、`run` サブコマンドと同等のジョブを起動して `RunData` を返さなければならない（MUST）。`wait` を受け付けてはならない（MUST NOT）。
`POST /exec` のレスポンスは CLI `run` と同じ inline output field（`stdout`/`stderr` と range/total bytes）を返さなければならない（MUST）。

#### Scenario: ジョブ起動成功

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{"command": ["echo", "hi"]}` を送る
**Then**: HTTP 200 かつ `job_id` を含む JSON が返る

#### Scenario: command フィールド欠落

**Given**: `agent-exec serve` が起動している
**When**: `POST /exec` に `{}` を送る
**Then**: HTTP 400 かつ `ok=false` の JSON が返る

### Requirement: GET /status/:id によるジョブ状態取得

`GET /status/:id` は `status` サブコマンドと同等の応答を HTTP 200 で返さなければならない（MUST）。job_id が存在しない場合は HTTP 404 を返さなければならない（MUST）。

#### Scenario: 状態取得

**Given**: `POST /exec` でジョブを起動した
**When**: `GET /status/<job_id>` をリクエストする
**Then**: HTTP 200 かつ `state` フィールドを含む JSON が返る

#### Scenario: 存在しない job_id

**Given**: `agent-exec serve` が起動している
**When**: `GET /status/nonexistent` をリクエストする
**Then**: HTTP 404 かつ `error.code="job_not_found"` を含む JSON が返る

### Requirement: GET /tail/:id によるログ末尾取得

`GET /tail/:id` は `tail` サブコマンドと同等の応答を返さなければならない（MUST）。

#### Scenario: ログ末尾取得

**Given**: 完了したジョブが存在する
**When**: `GET /tail/<job_id>` をリクエストする
**Then**: HTTP 200 かつ `stdout` フィールドを含む JSON が返る
**And**: `stdout_range` と `stdout_total_bytes` が含まれる

### Requirement: GET /wait/:id による完了待機

`GET /wait/:id` は `wait` サブコマンドと同等の動作をし、ジョブが終端状態になるまで待機してから応答しなければならない（MUST）。

#### Scenario: 完了まで待機

**Given**: 実行中のジョブが存在する
**When**: `GET /wait/<job_id>` をリクエストする
**Then**: ジョブ終了後に HTTP 200 かつ終端状態を含む JSON が返る

### Requirement: POST /kill/:id によるジョブ停止

`POST /kill/:id` は `kill` サブコマンドと同等のシグナル送信を行い、結果を返さなければならない（MUST）。

#### Scenario: ジョブ停止

**Given**: 実行中のジョブが存在する
**When**: `POST /kill/<job_id>` をリクエストする
**Then**: HTTP 200 かつ `ok=true` の JSON が返り、ジョブが停止する

### Requirement: JSON レスポンスのスキーマ準拠

すべてのエンドポイントのレスポンス JSON は `schema_version`, `ok`, `type` の共通フィールドを含まなければならない（MUST）。

#### Scenario: 共通フィールドの存在

**Given**: `GET /status/<job_id>` をリクエストする
**When**: レスポンスが返る
**Then**: `schema_version`, `ok`, `type` がすべて含まれる


### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須）と任意の `cwd`・`env`・`timeout`（秒、CLI `--timeout` と同じ秒単位契約） を受け取り、CLI の `run` と同等の既定待機・inline output 契約でジョブを起動して返さなければならない（MUST）。`wait` を受け付けてはならない（MUST NOT）。

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


### Requirement: POST /exec によるジョブ起動

`POST /exec` はリクエストボディの `command` フィールド（必須）と任意の `cwd`・`env`・`timeout`（秒、CLI `--timeout` と同じ秒単位契約） を受け取り、CLI の `run` と同等の既定待機・inline output 契約でジョブを起動して返さなければならない（MUST）。新規 job の `job_id` は hash-like 小文字 hex ID でなければならない（MUST）。`wait` を受け付けてはならない（MUST NOT）。

#### Scenario: POST /exec returns a hash-like job ID

Given `agent-exec serve` が起動している
When `POST /exec` に `{"command": ["echo", "hi"]}` を送る
Then HTTP 200 かつ `job_id` を含む JSON が返る
And `job_id` は `[0-9a-f]` のみで構成される固定長文字列である

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

## Requirements

### Requirement: 非 loopback bind の明示ガード

`agent-exec serve` の bind アドレスが loopback（`127.0.0.0/8` または `::1`）以外の場合、`--insecure` フラグを明示指定しない限り起動を拒否しなければならない（MUST）。拒否時は stderr に警告を出し、stdout に `{ok:false,error:{code:"serve_unsafe_bind"}}` を書いて exit code 1 で終了しなければならない（MUST）。

非 loopback bind を選択する場合は、`AGENT_EXEC_SERVE_TOKEN` 環境変数の設定を必須とする（MUST）。未設定時は `serve_unsafe_bind` と同様に起動拒否しなければならない（MUST）。

#### Scenario: non-loopback bind without --insecure is rejected

**Given**: a user executes `agent-exec serve --bind 0.0.0.0:19263`
**When**: the server attempts to start
**Then**: the process exits with code 1
**And**: stdout contains `error.code="serve_unsafe_bind"`

#### Scenario: non-loopback bind without token is rejected even with --insecure

**Given**: a user executes `agent-exec serve --bind 0.0.0.0:19263 --insecure`
**And**: `AGENT_EXEC_SERVE_TOKEN` is unset
**When**: the server attempts to start
**Then**: the process exits with code 1

### Requirement: Bearer トークン認証

`AGENT_EXEC_SERVE_TOKEN` 環境変数が設定されている場合、mutating エンドポイント（`POST /exec`・`POST /kill/:id`）は `Authorization: Bearer <token>` ヘッダを検証しなければならない（MUST）。ヘッダ欠落・値不一致は HTTP 401 と `error.code="unauthorized"` を返さなければならない（MUST）。

読み取り専用エンドポイント（`GET /health`・`GET /status/:id`・`GET /tail/:id`・`GET /wait/:id`）はトークン検証を要求しない（MAY）。

#### Scenario: POST /exec requires Bearer token when set

**Given**: `AGENT_EXEC_SERVE_TOKEN=secret` で serve が起動している
**When**: `POST /exec` に `Authorization` ヘッダ無しで送る
**Then**: HTTP 401 と `error.code="unauthorized"` が返る

#### Scenario: POST /exec accepts matching token

**Given**: `AGENT_EXEC_SERVE_TOKEN=secret` で serve が起動している
**When**: `POST /exec` を `Authorization: Bearer secret` 付きで送る
**Then**: HTTP 200 が返る

### Requirement: CORS の明示的 allow-origin

serve は既定で `Access-Control-Allow-Origin` を含むどの CORS ヘッダも返してはならない（MUST NOT）。`--allow-origin <ORIGIN>` が指定された場合に限り、当該 origin に対してのみ CORS ヘッダを返す（MUST）。wildcard `*` は受け付けてはならない（MUST NOT）。

#### Scenario: CORS headers are absent by default

**Given**: `agent-exec serve` が既定設定で起動している
**When**: `OPTIONS /exec` を preflight として送る
**Then**: `Access-Control-Allow-Origin` ヘッダは含まれない

#### Scenario: explicit allow-origin emits CORS header

**Given**: `agent-exec serve --allow-origin https://example.com` で起動している
**When**: `Origin: https://example.com` ヘッダ付きで `POST /exec` を送る
**Then**: レスポンスに `Access-Control-Allow-Origin: https://example.com` が含まれる

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
