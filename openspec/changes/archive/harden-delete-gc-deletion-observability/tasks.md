## Implementation Tasks

- [x] 1. `src/delete.rs` の single-delete / bulk-delete 経路に post-delete existence check を追加し、`action="deleted"` を返す条件を「`remove_dir_all` 成功」だけでなく「対象 path が command 完了時点で存在しない」まで引き上げる（verification: integration - `tests/integration.rs` に deleted 応答後の directory 不在確認を追加）。
- [x] 2. `src/gc.rs` の delete 経路にも同様の post-delete existence check を追加し、削除不能または削除結果が有効でない場合は `skipped`/failure reason にフォールバックする（verification: integration - `tests/integration.rs` の gc 系テストで deleted 応答 job の directory 不在を確認）。
- [x] 3. `delete --all` レスポンス schema に effective cwd scope を追加し、CLI 実装 (`src/delete.rs`, `src/schema.rs`) と README を同期する（verification: integration+manual - レスポンス JSON に cwd scope が含まれ、README 例/フィールド表が一致する）。
- [x] 4. `delete` / `gc` の observability を高める集計または per-job reason の表現を調整し、対象外・スキップ・削除成功の区別をレスポンスで明確化する（verification: integration - cwd mismatch / too_recent / running / deleted を区別するアサーションを追加）。
- [x] 5. canonical spec delta と README を更新し、`delete --all` が cwd-scoped であること、`gc` が retention-based root-wide であること、`deleted` は post-delete existence check 済みであることを明文化する（verification: manual - spec/README diff が実装契約と一致）。
- [x] 6. リポジトリ検証を実行する: `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all`（verification: command outputs show success）。

## Future Work

- 実運用で root の取り違えが頻発するなら、将来的に `delete` / `gc` に root provenance や out-of-scope count の追加集計をさらに広げる。
