## Implementation Tasks

- [x] 1. CLI にグローバル `--yaml` フラグを追加する（検証: `src/main.rs` の `Cli` で全サブコマンドから参照でき、`agent-exec --help` に表示される）
- [x] 2. 共通出力経路を format-aware に変更する（検証: `src/schema.rs` の `Response::print` / `ErrorResponse::print` が JSON 既定・YAML 任意を同じ経路で扱う）
- [x] 3. 各サブコマンドから選択された出力フォーマットを共通出力へ渡す（検証: `src/run.rs`, `src/status.rs`, `src/tail.rs`, `src/wait.rs`, `src/kill.rs`, `src/list.rs`, `src/schema_cmd.rs`, `src/install_skills.rs`, `src/main.rs` のエラー出力が `--yaml` 時に YAML を返す）
- [x] 4. YAML 直列化依存とテスト補助を追加する（検証: `Cargo.toml` に必要依存が追加され、統合テストから YAML をパースできる）
- [x] 5. 既定 JSON の後方互換テストと `--yaml` レスポンステストを追加する（検証: `tests/integration.rs` に少なくとも success/error/schema の YAML ケースがあり、JSON 既定ケースも維持される）
- [x] 6. ドキュメントと OpenSpec を更新する（検証: `README.md` と `openspec/specs/*/spec.md` または change delta が JSON 既定・YAML 任意を説明する）
- [x] 7. `cargo test --test integration` を実行する（検証: 対象統合テストが通る）
- [x] 8. `cargo test` を実行する（検証: 全テストが通る）

## Future Work

- 必要になれば `--output <format>` への拡張を別提案として扱う
- 外部 SDK で YAML 契約を明示的にサポートするかは利用実績を見て別途判断する
