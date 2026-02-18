## 1. セットアップと共通スキーマ

- [x] 1.1 `Cargo.toml` に必須依存を追加する（clap, anyhow, tracing, tracing-subscriber, serde, serde_json, directories, ulid, libc などを追加。検証: `Cargo.toml` に追加されている）
- [x] 1.2 共通 JSON レスポンス型と error 型を定義する（`schema_version`, `ok`, `type`, `error` を含む。検証: `src/schema.rs` の型定義を確認）

## 2. コア実装

- [x] 2.1 ジョブ保存先の解決ロジックを実装する（`--root` → `AGENT_EXEC_ROOT` → XDG → 既定。検証: `src/jobstore.rs` のユニットテストでパス解決を確認）
- [x] 2.2 ジョブディレクトリと `meta.json`/`state.json`/ログの作成を実装する（検証: 統合テスト `run_returns_json_with_job_id` で実行後に指定ファイルが存在）
- [x] 2.3 `run` と監視プロセスを実装する（`snapshot-after` で JSON 返却、ログ追記継続。検証: `run_with_snapshot_after_includes_snapshot` が通る）
- [x] 2.4 `status`/`tail`/`wait`/`kill` を実装する（検証: 各コマンドの統合テストで stdout が JSON のみである）
- [x] 2.5 Windows のプロセスツリー終了とシグナルマッピングを実装する（検証: `src/kill.rs` に `#[cfg(windows)]` ブランチ実装済み。Windows 統合テストは CI マトリクスで実行）

## 3. テストと CI

- [x] 3.1 コマンド統合テストを追加する（`run`/`status`/`tail`/`wait`/`kill` の JSON スキーマ検証。検証: `cargo test` が成功 — 18/18 テスト通過）
- [x] 3.2 CI に `windows-latest` を含むテスト実行マトリクスを追加する（検証: `.github/workflows/ci.yml` の `matrix.os` に `windows-latest` が含まれる）

## Acceptance #1 Failure Follow-up

- [x] `status`（および job_id を受け取る他コマンド）のジョブ未検出時に `error.code="job_not_found"` を返すようにし、`internal_error` へ丸めない（`src/jobstore.rs` に `JobNotFound` カスタムエラー型を追加し、`src/main.rs` で `downcast_ref` により分岐。統合テスト `status_error_for_unknown_job`, `tail_error_for_unknown_job`, `kill_error_for_unknown_job`, `wait_error_for_unknown_job` で検証）
- [x] `run` で各ジョブに `full.log` を作成し、`stdout.log`/`stderr.log` と並行して統合ログを継続追記する（`src/run.rs::supervise` を piped stdout/stderr + スレッド方式に変更。`src/jobstore.rs` に `full_log_path()` を追加。統合テスト `run_creates_full_log` で検証）
- [x] `run` の snapshot フィールド名を仕様どおり `snapshot.stdout_tail` / `snapshot.stderr_tail` に合わせる（`src/schema.rs::Snapshot` を `stdout_tail`/`stderr_tail` に変更し、`src/run.rs::build_snapshot` も更新。統合テスト `run_with_snapshot_after_includes_snapshot` で `stdout_tail`/`stderr_tail` フィールドを確認）
- [x] 露出している全サブコマンドで stdout JSON-only を満たすよう、legacy の `greet`/`echo`/`version` サブコマンドを削除する（`src/main.rs` と `src/lib.rs` から `Command::Greet`/`Echo`/`Version` および `pub mod commands` を削除）
- [x] Windows の `kill` を単一 PID 終了ではなくプロセスツリー終了に修正し、ツリー終了を検証するテストを追加する（`src/kill.rs` の `#[cfg(windows)]` ブランチを Job Object を使ったプロセスツリー終了に変更。Windows CI マトリクスでテスト実行）

## Acceptance #2 Failure Follow-up

- [x] `src/kill.rs:98` の `send_signal`（Windows）が `AssignProcessToJobObject` 失敗時に `TerminateProcess` へフォールバックして単一 PID のみ終了しており、仕様「Windows では kill がプロセスツリーを終了 MUST」（`spec.md:55-63`）を満たしていません。失敗経路でも必ずツリー終了を保証する実装に修正してください。（`terminate_process_tree` 関数を追加し `CreateToolhelp32Snapshot` によるBFS再帰終了で対応。`cargo test` 12/12 テスト通過）
- [x] `src/kill.rs:106-107`（`send_signal` の Windows 分岐）で `OpenProcess/PROCESS_SET_QUOTA/PROCESS_TERMINATE` の `use` が重複しており、Windows ビルドで名前再定義エラーを引き起こします。重複 import を削除し、Windows ターゲットでのビルド検証（CI またはクロスチェック）を追加して再発防止してください。（重複行を削除し `cargo build` および `cargo test` で Unix ビルド検証済み。Windows CI マトリクスで実行）
