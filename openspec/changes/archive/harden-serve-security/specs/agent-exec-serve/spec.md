## ADDED Requirements

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
