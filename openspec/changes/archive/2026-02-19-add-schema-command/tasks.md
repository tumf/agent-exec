## 1. CLI とレスポンス型の追加

- [x] 1.1 `src/main.rs` に `schema` サブコマンドを追加し、実行経路に接続する（検証: `src/main.rs` に `schema` サブコマンド定義がある）
- [x] 1.2 `src/schema.rs` に `SchemaData` を追加し、`type="schema"` のレスポンスを返せるようにする（検証: `SchemaData` の定義が確認できる）

## 2. スキーマデータの提供

- [x] 2.1 `schema/agent-exec.schema.json` を追加し、CLI レスポンスの JSON Schema を記述する（検証: 新規ファイルが存在する）
- [x] 2.2 `schema` コマンドで JSON Schema を読み込み、`schema_format` と共に返す（検証: `schema` 実装でファイル読み込みと出力が確認できる）

## 3. テスト

- [x] 3.1 `tests/integration.rs` に `agent-exec schema` のケースを追加する（検証: 新しいテストケースが追加されている）
- [x] 3.2 `schema_format` と `schema` の存在を検証するアサーションを追加する（検証: アサーションが追加されている）
