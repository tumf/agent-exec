## 0. 既存挙動の特徴化

- [x] 0.1 `cargo test --test integration run_returns_json_with_job_id` が成功することを確認する（検証: テストが通る）
- [x] 0.2 `cargo test --test integration error_response_has_retryable_field` が成功することを確認する（検証: テストが通る）

## 1. JSON 出力処理の共通化

- [x] 1.1 共通の JSON 出力ヘルパーを追加し `Response::print` と `ErrorResponse::print` から利用する（検証: `src/schema.rs` に共通ヘルパーがあり、両方の `print` が参照している）
- [x] 1.2 既存の JSON シリアライズ結果に変更がないことを確認する（検証: 主要テストの stdout JSON が引き続きパース可能）

## 2. 回帰確認

- [x] 2.1 `cargo test --test integration run_returns_json_with_job_id` を再実行する（検証: テストが通る）
- [x] 2.2 `cargo test --test integration error_response_has_retryable_field` を再実行する（検証: テストが通る）
