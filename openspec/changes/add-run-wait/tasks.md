## 1. CLI とスキーマ拡張

- [ ] 1.1 `src/main.rs` に `run --wait` フラグを追加する（検証: `src/main.rs` に `--wait` の定義がある）
- [ ] 1.2 `src/schema.rs` の `RunData` に `exit_code` / `finished_at` / `final_snapshot` を追加する（検証: `RunData` 定義で該当フィールドが確認できる）

## 2. 実行ロジック

- [ ] 2.1 `src/run.rs` の `run` 実行経路に `--wait` の分岐を追加し、終端状態まで待機する（検証: `run` の実装で待機処理が `--wait` で有効になる）
- [ ] 2.2 終了時点の `final_snapshot` を生成する処理を追加する（検証: `final_snapshot` 生成が `--wait` の経路で呼ばれる）

## 3. テスト

- [ ] 3.1 `tests/integration.rs` に `run --wait` のシナリオを追加する（検証: 新しいテストケースが追加されている）
- [ ] 3.2 `run --wait` のレスポンスに `finished_at` と `final_snapshot` が含まれることを検証する（検証: 該当アサーションが追加されている）
