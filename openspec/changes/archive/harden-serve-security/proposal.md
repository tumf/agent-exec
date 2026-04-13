---
change_type: implementation
priority: high
dependencies: []
references:
  - src/serve.rs
  - openspec/specs/agent-exec-serve/spec.md
---

# 変更提案: serve のセキュリティ強化（非 loopback bind ガード / 認証 / CORS）

## Problem / Context

`src/serve.rs` には現在、認証・CORS・非 loopback bind 警告が**一切無い**（全行調査済、middleware 無し）。`POST /exec` は任意コマンド実行エンドポイントであり、`--bind 0.0.0.0` 指定時に即 RCE 化する。`agent-exec-serve/spec.md` にもセキュリティ要件が記載されていない。

## Proposed Solution

### Bind ガード
- 既定 bind `127.0.0.1:19263` は維持。
- 非 loopback bind（`0.0.0.0`, `::`, non-127/0/8 など）を指定する場合は `--insecure` フラグを明示必須とする。未指定時は起動拒否（`error.code="serve_unsafe_bind"`）。

### 認証トークン
- 環境変数 `AGENT_EXEC_SERVE_TOKEN` が設定されていれば、すべての mutating エンドポイント（`POST /exec`・`POST /kill/*`）で `Authorization: Bearer <token>` を要求（MUST）。ヘッダ欠落・不一致は HTTP 401 / `error.code="unauthorized"`。
- 非 loopback bind を選択する場合は `AGENT_EXEC_SERVE_TOKEN` の設定を必須とする（未設定時は起動拒否）。

### CORS
- 既定は Access-Control-Allow-Origin を返さない（同一 origin のみ）。
- `--allow-origin <ORIGIN>` を明示指定した場合のみ当該 origin を許可。wildcard `*` は許可しない。

## Acceptance Criteria

- `--bind 0.0.0.0:19263` だけで起動すると拒否される。
- `--bind 0.0.0.0:19263 --insecure` かつ `AGENT_EXEC_SERVE_TOKEN` 未設定でも拒否される。
- token 設定あり & ヘッダなしで `POST /exec` が 401。
- `--allow-origin https://example.com` 指定時のみ、その origin に CORS ヘッダが返る。

## Out of Scope

- mTLS / TLS 終端は本提案に含めない（reverse proxy 前提）。
- rate limiting は別提案。
