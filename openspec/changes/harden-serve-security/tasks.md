## Implementation Tasks

- [ ] Task 1: `serve` 起動時に bind アドレスを解析、loopback 以外でかつ `--insecure` 未指定なら `serve_unsafe_bind` で起動拒否
  - verification: unit — `src/serve.rs`
- [ ] Task 2: 非 loopback bind 時は `AGENT_EXEC_SERVE_TOKEN` 未設定でも起動拒否
  - verification: unit
- [ ] Task 3: axum middleware 追加: `AGENT_EXEC_SERVE_TOKEN` が設定されていれば `POST /exec` / `POST /kill/*` に Bearer 認証を要求
  - verification: unit — tower layer テスト
- [ ] Task 4: 認証失敗時は HTTP 401 / `{ok:false,error:{code:"unauthorized"}}` を返す
  - verification: unit
- [ ] Task 5: `--allow-origin <ORIGIN>` フラグ追加、指定 origin にのみ CORS ヘッダを返す。wildcard は拒否
  - verification: unit — CORS layer テスト
- [ ] Task 6: integration test: `--bind 0.0.0.0:0 --insecure` + token 無しで起動拒否
  - verification: integration — `tests/integration.rs` serve セクション
- [ ] Task 7: integration test: token 設定あり・ヘッダ無しで 401
  - verification: integration
- [ ] Task 8: integration test: `--allow-origin https://example.com` 指定時のみ CORS ヘッダが付く
  - verification: integration

## Specification Tasks

- [ ] Promote `specs/agent-exec-serve/spec.md` delta — bind ガード / auth / CORS

## Future Work

- TLS 終端は reverse proxy 側に委譲する運用を README に記載。
- Rate limiting は別提案。
