## Implementation Tasks

- [ ] Task 1: `ExecRequest` (`src/serve.rs:125-132`) に `wait` / `until` / `max_bytes` / `timeout` を追加し、`timeout_ms` を削除する
  - verification: unit — serde テスト
- [ ] Task 2: `POST /exec` ハンドラで新フィールドを `observe_inline_output` に渡す
  - verification: unit
- [ ] Task 3: integration test: `until=1` で短時間観測が可能
  - verification: integration — `tests/integration.rs` serve 系
- [ ] Task 4: integration test: `wait=false` で launch-only 返却
  - verification: integration
- [ ] Task 5: integration test: `max_bytes=1024` で output 切り詰め
  - verification: integration
- [ ] Task 6: integration test: `timeout_ms` 指定は HTTP 400（unknown field）
  - verification: integration

## Future Work

- HTTP API 契約の break change を CHANGELOG/README に反映（`define-schema-version-policy` 後に自動化）。
