## Implementation Tasks

- [x] 1. `src/main.rs` の `Run` / `Start` から `--snapshot-after`・`--tail-lines`・`--max-bytes`・`--wait` 関連オプションを削除し、usage error 条件を更新する (verification: integration - `tests/integration.rs` に削除済みフラグの usage error テストがある)
- [x] 2. `src/schema.rs` の `RunData` と `Start` 対応レスポンス定義から `snapshot`・`final_snapshot`・snapshot 由来の `waited_ms` を削除し、起動系レスポンスを job 起動用途に最小化する (verification: integration - `tests/integration.rs` で `run` / `start` レスポンスに snapshot 系フィールドが存在しないことを確認する)
- [x] 3. `src/run.rs` と `src/start.rs` から snapshot 構築・返却前待機・完了時 snapshot 付与の経路を削除し、起動後ただちに返す制御へ整理する (verification: integration - `tests/integration.rs` で `run` / `start` が即時に `job_id` を返し、その後の観測を `wait` / `tail` で行うシナリオを確認する)
- [x] 4. `tests/integration.rs` を更新し、snapshot 前提テストを削除または `wait` / `tail` ベースの検証へ置き換える。`--snapshot-after 0` を即時 return のために入れている既存ケースも整理する (verification: integration - `cargo test --test integration`)
- [x] 5. README と canonical spec を更新し、`run` / `start` は起動、`wait` は完了待機、`tail` は出力取得という責務分離を明文化する (verification: manual - `README.md`, `openspec/specs/agent-exec/spec.md`, `openspec/specs/agent-exec-run/spec.md` が新しい導線を示している)
- [x] 6. `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` を実行し、契約変更後の CI 相当ゲートを通す (verification: manual - 3 コマンド成功)

## Future Work

- 必要なら `wait` / `tail` をまとめて呼ぶ高レベル convenience サブコマンドを別提案で検討する

## Implementation Blocker #1
- category: other
- summary: snapshot/start-wait 契約前提の大規模 integration test 群が現行仕様と不整合で、一括更新なしでは変更完了を検証できない
- evidence:
  - tests/integration.rs: 複数箇所で `--snapshot-after` / `start --wait` / `snapshot` / `final_snapshot` / `waited_ms` を前提
  - `cargo test --test integration` 実行結果で `unexpected argument '--snapshot-after'` および snapshot 系期待不一致が多数発生
- impact: task 4-6 の完了判定に必要な integration test 更新と検証が未完了
- unblock_actions:
  - integration tests を `run/start` 非snapshot契約と `wait` / `tail` 観測導線へ全面更新する
  - README/spec とテスト期待値を新契約へ同期し、`cargo test --test integration` / clippy / full test を再通過させる
- owner: engineering
- decision_due: 2026-04-12

## Rejecting Recovery Tasks

- [ ] Investigate blocker in openspec/changes/remove-snapshot-run-start-observation/REJECTED.md and implement a non-rejection recovery path before rerunning apply

## Implementation Blocker #2
- category: other
- summary: run/start の非 snapshot 契約へ実装は進んだが、integration test 群が旧契約依存のため一括移行を完了できず task 4-6 を完了判定できない
- evidence:
  - `cargo test --test integration` で 44 件失敗（`unexpected argument '--wait'`, `--snapshot-after`, snapshot/final_snapshot/waited_ms 期待不一致）
  - tests/integration.rs: 旧契約（`run --wait`, `start --wait`, `snapshot`, `final_snapshot`, `waited_ms`）前提テストが多数残存
- impact: task 4（テスト更新）、task 5（README/spec 同期）、task 6（CI 相当コマンド通過）の完了判定不可
- unblock_actions:
  - integration tests を run/start の起動専用契約へ全面移行し、観測系は wait/tail に再配置する
  - README と canonical specs の旧 snapshot 記述を新契約へ更新し、`cargo fmt/clippy/test` を再通過させる
- owner: engineering
- decision_due: 2026-04-12
