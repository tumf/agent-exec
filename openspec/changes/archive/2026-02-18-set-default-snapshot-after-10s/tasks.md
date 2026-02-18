## 1. 既定値の変更

- [x] 1.1 `run` の `snapshot-after` 既定値を 10,000ms に更新する（検証: `src/main.rs` の clap デフォルトが 10000）
- [x] 1.2 `RunOpts` の既定値を 10,000ms に揃える（検証: `src/run.rs` の `RunOpts::default` が 10000）

## 2. テスト更新

- [x] 2.1 既定 `run` が `snapshot` を返し `waited_ms <= 10000` であることを確認するテストを追加する（検証: `tests/integration.rs` に該当アサートがある）
- [x] 2.2 既定待機による影響があるテストに `--snapshot-after 0` を明示して即時返却させる（検証: `tests/integration.rs` の該当テストで引数が更新されている）
