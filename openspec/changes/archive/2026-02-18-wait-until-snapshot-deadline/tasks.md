## 1. 待機ロジックの更新

- [x] 1.1 `run` の待機ループから出力検知による早期終了を削除する（検証: `src/run.rs` の待機条件が「期限 or ジョブ終了」のみになっている）
- [x] 1.2 `waited_ms` が `snapshot-after` 以上になることを確認できるよう測定ロジックを維持する（検証: `src/run.rs` で `waited_ms` が `snapshot-after` を下回らないことが読み取れる）

## 2. テスト更新

- [x] 2.1 即時出力 + 継続実行のテストを追加し `waited_ms >= snapshot-after` を検証する（検証: `tests/integration.rs` に `printf` + `sleep` を使ったアサートがある）
