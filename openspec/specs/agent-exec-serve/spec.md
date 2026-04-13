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
