## 1. 共有ログメトリクスの基盤追加

- [x] 1.1 `src/jobstore.rs` に末尾取得と bytes メトリクスを返す共有ヘルパーを追加する。検証: `src/jobstore.rs` に新しいヘルパーと返却構造体が存在し、`encoding` が `utf-8-lossy` である

## 2. 既存フローへの組み込み

- [x] 2.1 `src/run.rs` の `build_snapshot` を共有ヘルパー経由に置換する。検証: `src/run.rs` の snapshot 生成がヘルパー呼び出しを利用している
- [x] 2.2 `src/tail.rs` を共有ヘルパー経由に置換する。検証: `src/tail.rs` の `TailData` 生成がヘルパー呼び出しを利用している

## 3. 回帰確認

- [x] 3.1 `cargo test --all` を実行する。検証: すべてのテストが成功する
