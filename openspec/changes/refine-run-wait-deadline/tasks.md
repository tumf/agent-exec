## Implementation Tasks

- [ ] 1. `src/main.rs` の `Run` に `--until <ms>` と `--forever` を追加し、`--wait` 必須・相互排他の clap 制約を定義する (verification: `src/main.rs` の `Run` 定義で `until` / `forever` が確認でき、usage error 条件をテストで再現できる)
- [ ] 2. `src/main.rs` の `Wait` に `--until <ms>` と `--forever` を追加し、相互排他の clap 制約を定義する。既存 `--timeout-ms` は `--until` のエイリアスまたは deprecated として残すか除去する (verification: `src/main.rs` の `Wait` 定義で `until` / `forever` が確認でき、usage error 条件をテストで再現できる)
- [ ] 3. `src/run.rs` の `run_snapshot_wait` 経路に待機期限を追加し、`run --wait` の既定を 30,000ms、`--until` で上書き、`--forever` で無制限待機にする (verification: `src/run.rs` で `--wait` 時の deadline 計算が `30000ms` / `until` / `forever` を反映している)
- [ ] 4. `src/wait.rs` の `WaitOpts` と `execute` を `--until` / `--forever` 体系に移行し、既定待機上限を 30,000ms にする (verification: `src/wait.rs` で deadline が `30000ms` / `until` / `forever` に基づいて計算される)
- [ ] 5. 待機期限到達時の `run` および `wait` レスポンスを実装し、ジョブは継続実行したまま非終端 state を返す (verification: `tests/integration.rs` に `run --wait --until` と `wait --until` で未完了ジョブが `running|created` のまま返るシナリオがある)
- [ ] 6. `openspec/specs/agent-exec-run/spec.md` を更新し、`run --wait` の既定 30 秒・`--until`・`--forever`・`--timeout` との役割分離を canonical requirement に反映する (verification: canonical spec に新しい待機仕様が記述されている)
- [ ] 7. `tests/integration.rs` に `run --wait` 系 (既定 30 秒、`--until`、`--forever`、clap 排他) と `wait` 系 (既定、`--until`、`--forever`、clap 排他) の統合テストを追加する (verification: `cargo test --test integration` で対象シナリオが通る)
- [ ] 8. `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` を実行して CI 相当の品質ゲートを通す (verification: コマンド実行ログが成功する)

## Future Work

- `start` サブコマンドの `--wait` にも同じ `--until` / `--forever` 体系を適用する（別提案）
