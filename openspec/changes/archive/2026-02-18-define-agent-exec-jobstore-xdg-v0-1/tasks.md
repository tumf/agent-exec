## 1. パス解決とジョブ構造

- [x] 1.1 ジョブ保存先の解決ロジックを実装する（検証: 解決順序のユニットテストが追加される）
- [x] 1.2 ジョブディレクトリとログ/JSONファイルの生成を実装する（検証: 実行後に `meta.json`/`state.json`/ログが生成される）

## 2. meta/state の内容と書き込み

- [x] 2.1 `meta.json` の必須フィールドと env_keys のみ保存を実装する（検証: `meta.json` に env 値が含まれない）
- [x] 2.2 `state.json` の必須フィールドと更新を実装する（検証: `state.json` に `updated_at` が含まれる）
- [x] 2.3 `meta.json` と `state.json` の原子的な書き込みを実装する（検証: 一時ファイル経由の書き込みが行われる）

## Acceptance #1 Failure Follow-up

- [x] `meta.json` / `state.json` を仕様どおりの必須フィールド構造（`job.id`, `job.status`, `job.started_at`, `result.exit_code`, `result.signal`, `result.duration_ms`, `updated_at`）で永続化する。`Option` 値でもキーを省略せず `null` で出力する。（`schema.rs` の `JobState` から `exit_code`/`signal`/`duration_ms` の `skip_serializing_if` を削除）
- [x] `run` 実行直後にジョブディレクトリへ `stdout.log` / `stderr.log` / `full.log` が必ず存在するようにする（`run.rs` でジョブディレクトリ作成直後に空ファイルを事前作成）。統合テスト `run_creates_all_log_files_immediately` で検証済み。
- [x] `cargo test` が安定して通るように、環境変数を変更する `jobstore` テストを直列化する（`ENV_LOCK: Mutex<()>` をモジュールレベルで定義し、各環境変数変更テストがロックを保持してから操作するようリファクタリング）。`resolve_root_env_var` の失敗を解消済み。

## Acceptance #2 Failure Follow-up

- [x] `meta.json` / `state.json` の永続化スキーマを仕様のネスト構造に合わせる（`meta.json` は `job.id`、`state.json` は `job.id` / `job.status` / `job.started_at` と `result.exit_code` / `result.signal` / `result.duration_ms` をトップレベル `updated_at` と併せて出力）。`src/schema.rs` の `JobMeta` / `JobState` 型を再設計し、`src/run.rs` の初期状態・終了状態書き込み、`src/jobstore.rs` の read/write と `tests/integration.rs` の検証を新構造へ更新する。
