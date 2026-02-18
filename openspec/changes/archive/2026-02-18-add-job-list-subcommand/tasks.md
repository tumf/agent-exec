## 1. CLI 入口の追加

- [x] 1.1 `Command` に `list` サブコマンドと `--root`/`--limit` を追加する（検証: `src/main.rs` に `List` 変種と引数定義がある）
- [x] 1.2 `list` の実行パスを `run()` の match に配線する（検証: `src/main.rs` で `agent_shell::list::execute(...)` が呼ばれる）

## 2. スキーマと list 実装

- [x] 2.1 `list` のレスポンス型を `schema` に追加する（検証: `src/schema.rs` に `ListData` と `JobSummary` が定義されている）
- [x] 2.2 `list` モジュールを実装する（検証: `src/list.rs` の `execute()` が `Response::new("list", ...)` を返し、root/limit/並び順/skip を処理する）
- [x] 2.3 `list` モジュールを公開する（検証: `src/lib.rs` に `pub mod list;` がある）

## 3. 統合テスト

- [x] 3.1 `list` の統合テストを追加する（検証: `tests/integration.rs` に list のテストケースが追加されている）
- [x] 3.2 統合テストが通ることを確認する（検証: `cargo test --test integration`）
