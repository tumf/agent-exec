# Design: add-schema-command

## 目的
`agent-exec schema` で CLI レスポンスの JSON Schema を取得できるようにし、外部統合の初期実装と変更検知を容易にする。

## 主要な設計判断
- **静的スキーマ採用**: `schema/agent-exec.schema.json` をリポジトリ内に同梱し、`schema` コマンドはそれを読み込んで返す。
- **JSON-only 出力**: 既存契約に従い、stdout は JSON オブジェクト 1 つのみ。

## レスポンス形
`schema` の成功レスポンスは `type="schema"` を持ち、以下を含める。
- `schema_format`: `json-schema-draft-07`
- `schema`: JSON Schema 本体（オブジェクト）
- `generated_at`: 生成/更新時刻（RFC3339）

## 実装の影響範囲
- CLI: `src/main.rs` に `schema` サブコマンドを追加
- Schema: `src/schema.rs` に `SchemaData` を追加
- データ: `schema/agent-exec.schema.json` を新規追加
- テスト: `tests/integration.rs` に `schema` の JSON-only 検証を追加
