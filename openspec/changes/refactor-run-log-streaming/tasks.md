## 0. 既存挙動の特徴化

- [ ] 0.1 `cargo test --test integration run_creates_full_log` が成功することを確認する（検証: テストが通る）
- [ ] 0.2 `cargo test --test integration run_with_snapshot_after_includes_snapshot` が成功することを確認する（検証: テストが通る）

## 1. ログストリーミング共通化

- [ ] 1.1 stdout/stderr の読み取り・`full.log` 書き込みを行う共通ヘルパーを抽出する（検証: `src/run.rs` で stdout/stderr が同一のヘルパーを利用している）
- [ ] 1.2 既存のバッファサイズと行分割ロジックを保持する（検証: バッファ長・改行処理がリファクタ前と同一であることを確認）

## 2. 回帰確認

- [ ] 2.1 `cargo test --test integration run_creates_full_log` を再実行する（検証: テストが通る）
- [ ] 2.2 `cargo test --test integration run_with_snapshot_after_includes_snapshot` を再実行する（検証: テストが通る）
