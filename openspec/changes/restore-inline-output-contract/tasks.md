## Implementation Tasks

- [ ] 1. `src/main.rs` の `Run` / `Start` に `--wait`, `--until`, `--forever`, `--no-wait` と head 取得サイズ指定を追加し、既定を `--wait --until 10` 相当に戻す (verification: integration - `tests/integration.rs` に `run` / `start` の既定待機・`--no-wait`・排他条件の CLI テストを追加)
- [ ] 2. `src/schema.rs` と関連レスポンス構造を更新し、`run` / `start` / `tail` / serve で `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を canonical field として返すようにする (verification: integration - `tests/integration.rs`, `tests/serve_integration.rs` で field 名と range 値を確認)
- [ ] 3. `src/run.rs` と `src/start.rs` に head 取得ロジックを実装し、待機予算内に観測できた log 先頭 bytes を UTF-8 lossy + raw byte range で返すようにする (verification: integration - 短命コマンド、長命コマンド、`--no-wait`, range 境界ケースを `tests/integration.rs` で確認)
- [ ] 4. `src/tail.rs`, `src/jobstore.rs`, `src/serve.rs` を更新し、tail も同じ range 契約で末尾 bytes を返すようにする。`POST /exec` は CLI `run` 相当、`GET /tail/:id` は CLI `tail` 相当に揃える (verification: integration - `tests/integration.rs` と `tests/serve_integration.rs` で CLI/HTTP の shape 一致を確認)
- [ ] 5. launch-only 前提の既存テストを置き換え、`run_rejects_removed_wait_flag` や `snapshot` 不在前提テストを inline output 契約の回帰テストへ更新する (verification: integration - `cargo test --test integration`, `cargo test --test serve_integration`)
- [ ] 6. canonical spec と README / skills / help 文言を更新し、`run` / `start` の既定 10 秒待機、`--no-wait`, head/tail の役割分担、range 契約を明記する (verification: manual - `openspec/specs/agent-exec/spec.md`, `openspec/specs/agent-exec-run/spec.md`, `openspec/specs/agent-exec-serve/spec.md`, README, skills が一致)
- [ ] 7. `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all` を実行し、契約変更後の CI 相当ゲートを通す (verification: manual - 3 コマンド成功)

## Future Work

- `status` / `wait` でも同一 range 契約を返す高レベル観測 API が必要なら別 proposal で扱う
