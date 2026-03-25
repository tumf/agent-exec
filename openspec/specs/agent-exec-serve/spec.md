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

`POST /exec` はリクエストボディの `command` フィールド（必須）と任意の `cwd`・`env`・`timeout_ms`・`wait` を受け取り、`run` サブコマンドと同等のジョブを起動して `RunData` を返さなければならない（MUST）。

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
