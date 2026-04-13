## Implementation Tasks

- [x] Task 1: `ErrorDetail` (`src/schema.rs:75-81`) に `details: Option<serde_json::Value>`（`skip_serializing_if = "Option::is_none"`）を追加
  - verification: unit — serde テスト
- [x] Task 2: `AmbiguousJobId` のハンドリング（`src/main.rs:692` 付近）で `ErrorResponse` 構築時に `details.candidates` と `details.truncated` を埋める
  - verification: unit — error mapping テスト
- [x] Task 3: 候補列挙の上限を 20 件に拡張（`src/jobstore.rs:40-50`）
  - verification: unit
- [x] Task 4: integration test: CLI で ambiguous prefix を投げたとき `error.details.candidates` が返る
  - verification: integration
- [x] Task 5: integration test: HTTP `GET /status/<prefix>` でも同じ
  - verification: integration

## Future Work

- なし
