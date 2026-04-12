## Implementation Tasks

- [x] 1. `src/main.rs` の `Run` / `Start` から `--snapshot-after`・`--tail-lines`・`--max-bytes`・`--wait` 関連オプションを削除し、usage error 条件を更新する (verification: integration - `tests/integration.rs` に削除済みフラグの usage error テストがある)
- [x] 2. `src/schema.rs` の `RunData` と `Start` 対応レスポンス定義から `snapshot`・`final_snapshot`・snapshot 由来の `waited_ms` を削除し、起動系レスポンスを job 起動用途に最小化する (verification: integration - `tests/integration.rs` で `run` / `start` レスポンスに snapshot 系フィールドが存在しないことを確認する)
- [x] 3. `src/run.rs` と `src/start.rs` から snapshot 構築・返却前待機・完了時 snapshot 付与の経路を削除し、起動後ただちに返す制御へ整理する (verification: integration - `tests/integration.rs` で `run` / `start` が即時に `job_id` を返し、その後の観測を `wait` / `tail` で行うシナリオを確認する)
- [x] 4. `tests/integration.rs` を更新し、snapshot 前提テストを削除または `wait` / `tail` ベースの検証へ置き換える。`--snapshot-after 0` を即時 return のために入れている既存ケースも整理する (verification: integration - `cargo test --test integration`)
- [x] 5. README と canonical spec を更新し、`run` / `start` は起動、`wait` は完了待機、`tail` は出力取得という責務分離を明文化する (verification: manual - `README.md`, `openspec/specs/agent-exec/spec.md`, `openspec/specs/agent-exec-run/spec.md` が新しい導線を示している)
- [x] 6. `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` を実行し、契約変更後の CI 相当ゲートを通す (verification: manual - 3 コマンド成功)

## Future Work

- 必要なら `wait` / `tail` をまとめて呼ぶ高レベル convenience サブコマンドを別提案で検討する
