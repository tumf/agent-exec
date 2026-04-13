## Implementation Tasks

- [x] 1. 共通 job ID 生成器を導入し、`run` / `create` / `serve /exec` の新規 job 作成を小文字 hex ID に切り替える（verification: integration - `tests/integration.rs` と `tests/serve_integration.rs` に新形式生成の検証を追加）
- [x] 2. 同一 root 配下での ID 衝突回避ループを実装し、job directory 名の一意性を保証する（verification: unit - `src/jobstore.rs` か生成器モジュールの単体テストで衝突時再生成を確認）
- [x] 3. `list` の job summary に `short_job_id` を追加し、常用表示を先頭 7 文字へ統一する（verification: integration - `agent-exec list` の JSON shape 検証を更新）
- [x] 4. exact match / 一意 prefix / ambiguous error の解決が新形式 ID と既存 ULID の混在 root でも維持されることを確認する（verification: unit+integration - `src/jobstore.rs` と `tests/integration.rs` で新旧混在ケースを追加）
- [x] 5. HTTP `POST /exec` / `GET /status/:id` / `GET /tail/:id` / `GET /wait/:id` / `POST /kill/:id` が新形式 ID と prefix 指定で継続動作することを検証する（verification: integration - `tests/serve_integration.rs`）
- [x] 6. 仕様変更に合わせて canonical behavior を反映するテストと docs を最小更新し、`cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` を通す（verification: unit+integration - 上記コマンド実行）

## Future Work

- 必要であれば shell completion の help 表示に `short_job_id` を併記する追加改善
- 既存 README の job ID 説明文を、proposal 実装と同時に見直すか別 change として分離する判断
