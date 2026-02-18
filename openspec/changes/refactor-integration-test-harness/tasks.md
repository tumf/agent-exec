## 0. 既存挙動の特徴化

- [ ] 0.1 `cargo test --test integration run_returns_json_with_job_id` が成功することを確認する（検証: テストが通る）
- [ ] 0.2 `cargo test --test integration status_returns_json_for_existing_job` が成功することを確認する（検証: テストが通る）

## 1. テストハーネスの導入

- [ ] 1.1 `tempdir` 作成と `AGENT_EXEC_ROOT` 設定をまとめたヘルパーを追加する（検証: `tests/integration.rs` で複数テストがヘルパーを利用している）
- [ ] 1.2 コマンド実行ヘルパーを整理し、共通のエラー出力整形を維持する（検証: 既存の panic メッセージ構造が保たれている）

## 2. 回帰確認

- [ ] 2.1 `cargo test --test integration` を再実行する（検証: テストが通る）
