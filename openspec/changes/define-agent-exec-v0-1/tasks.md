## 1. セットアップと共通スキーマ

- [ ] 1.1 `Cargo.toml` に必須依存を追加する（clap, anyhow, tracing, tracing-subscriber, serde, serde_json, directories, ulid などを追加。検証: `Cargo.toml` に追加されている）
- [ ] 1.2 共通 JSON レスポンス型と error 型を定義する（`schema_version`, `ok`, `type`, `error` を含む。検証: 型定義ファイルを確認）

## 2. コア実装

- [ ] 2.1 ジョブ保存先の解決ロジックを実装する（`--root` → `AGENT_EXEC_ROOT` → XDG → 既定。検証: ユニットテストでパス解決を確認）
- [ ] 2.2 ジョブディレクトリと `meta.json`/`state.json`/ログの作成を実装する（検証: 実行後に指定ファイルが存在）
- [ ] 2.3 `run` と監視プロセスを実装する（`snapshot-after` で JSON 返却、ログ追記継続。検証: `run` が JSON を返し、`state.json` が更新され続ける）
- [ ] 2.4 `status`/`tail`/`wait`/`kill` を実装する（検証: 各コマンドの stdout が JSON のみである）
- [ ] 2.5 Windows のプロセスツリー終了とシグナルマッピングを実装する（検証: Windows 用の統合テストで `kill` が完了する）

## 3. テストと CI

- [ ] 3.1 コマンド統合テストを追加する（`run`/`status`/`tail`/`wait`/`kill` の JSON スキーマ検証。検証: `cargo test` が成功）
- [ ] 3.2 CI に `windows-latest` を含むテスト実行マトリクスを追加する（検証: `.github/workflows/*` に Windows が含まれる）
