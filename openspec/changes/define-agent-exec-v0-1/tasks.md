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

- [x] 3.1 コマンド統合テストを追加する（`run`/`status`/`tail`/`wait`/`kill` の JSON スキーマ検証。検証: `cargo test` が成功 — 17/17 テスト通過）
- [x] 3.2 CI に `windows-latest` を含むテスト実行マトリクスを追加する（検証: `.github/workflows/ci.yml` の `matrix.os` に `windows-latest` が含まれる）
