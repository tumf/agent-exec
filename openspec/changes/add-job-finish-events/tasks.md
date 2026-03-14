## Implementation Tasks

- [x] 1. `run` CLI に `--notify-command <json-argv>` と `--notify-file <path>` を追加し、`RunOpts` と supervisor 起動引数へ伝搬する（verification: `src/main.rs` と `src/run.rs` に新オプションがあり、`_supervise` まで値が渡る）
- [x] 2. 通知設定と completion event persisted model を追加する（verification: `src/schema.rs` に notification / completion event 用 struct があり、`meta.json` と `completion_event.json` の永続化形が定義されている）
- [x] 3. job directory helper を追加し、`completion_event.json` を原子的に書き込めるようにする（verification: `src/jobstore.rs` に completion event path/helper があり、既存の atomic write パターンで保存される）
- [x] 4. supervisor の terminal-state 処理で `job.finished` payload を生成し、state 書き込み後に 1 回だけ sink 配送する（verification: `src/run.rs` に terminal state 後の event 組み立てと配送処理があり、配送失敗でも `JobState` を上書きしない）
- [x] 5. command sink を shell 非依存で実装し、event JSON を stdin と `AGENT_EXEC_EVENT_PATH` / `AGENT_EXEC_JOB_ID` / `AGENT_EXEC_EVENT_TYPE` 環境変数で渡す（verification: `src/run.rs` に `Command::new(argv[0])` ベースの実装があり、shell 経由の文字列実行をしていない）
- [x] 6. file sink を NDJSON append で実装し、親ディレクトリ自動作成と 1 completion event あたり 1 行書き込みを保証する（verification: `src/run.rs` または専用 helper に append 実装があり、単一 event を 1 行で書く）
- [x] 7. 統合テストを追加して file sink / command sink / notification failure の挙動を固定する（verification: `tests/integration.rs` に追加テストがあり、`cargo test --test integration` で関連シナリオが通る）

## Future Work

- `webhook` sink を別 proposal で追加する
- delivery retry / backoff と idempotency key を別 proposal で設計する
- user-defined labels / metadata passthrough を必要になった時点で拡張する
