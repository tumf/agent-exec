## 1. スキーマ拡張

- [ ] 1.1 `run` の JSON に `waited_ms`/`elapsed_ms` と `stdout_log_path`/`stderr_log_path` を追加する（検証: `src/schema.rs` にフィールド定義がある）
- [ ] 1.2 `snapshot`/`tail` の JSON に `*_observed_bytes` と `*_included_bytes` を追加する（検証: `src/schema.rs` の `Snapshot` と `TailData` にフィールド定義がある）

## 2. 実装

- [ ] 2.1 `snapshot-after` の待機を最大 10,000ms にクランプし、実測 `waited_ms` と `elapsed_ms` を計測して `run` の JSON に含める（検証: `src/run.rs` で待機と計測が行われている）
- [ ] 2.2 `run` の `snapshot` と `tail` で `observed_bytes`/`included_bytes` を計算し JSON に含める（検証: `src/run.rs` と `src/tail.rs` で bytes メトリクスを設定している）
- [ ] 2.3 `run`/`tail` の JSON に `stdout_log_path`/`stderr_log_path` の絶対パスを含める（検証: `src/run.rs` と `src/tail.rs` でパスがセットされている）

## 3. テスト

- [ ] 3.1 統合テストに `run` の `waited_ms`/`elapsed_ms` と `snapshot` bytes メトリクスの存在確認を追加する（検証: `tests/integration.rs` の該当テストでアサートがある）
- [ ] 3.2 統合テストに `tail` のログパスと bytes メトリクスの存在確認を追加する（検証: `tests/integration.rs` の該当テストでアサートがある）
- [ ] 3.3 `snapshot-after` が 10 秒にクランプされることを検証するテストを追加する（検証: `tests/integration.rs` に `waited_ms <= 10000` のアサートがある）
