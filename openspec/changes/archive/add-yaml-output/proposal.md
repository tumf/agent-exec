# 変更提案: add-yaml-output

## Problem/Context

agent-exec は現在 stdout を JSON 1 オブジェクトに固定しているため、CLI から人間が読むときや YAML を前提にした既存ツールチェーンへつなぐときに変換が必要になる。

現状の実装では `src/schema.rs` の `Response::print` / `ErrorResponse::print` が JSON 出力に固定され、`README.md` と OpenSpec も JSON-only 契約を前提としている。`schema` サブコマンドも JSON エンベロープ前提で説明されているため、YAML を追加するには CLI 契約・出力実装・ドキュメント・統合テストをまとめて更新する必要がある。

## Proposed Solution

`agent-exec` にグローバルフラグ `--yaml` を追加し、すべての成功レスポンスとエラーレスポンスを YAML でも出力できるようにする。既定値は従来どおり JSON とし、既存の機械連携を壊さない。

提案スコープ:
- `src/main.rs` にグローバル `--yaml` フラグを追加する
- `src/schema.rs` の共通出力経路を format-aware にして JSON/YAML を切り替える
- `schema` を含む全サブコマンドで同一のレスポンス内容を YAML でも表現できるようにする
- `README.md` と関連仕様を更新し、JSON 既定・YAML 任意の契約を明文化する
- `tests/integration.rs` に既定 JSON の後方互換テストと `--yaml` 回帰テストを追加する

## Acceptance Criteria

- `agent-exec --yaml <subcommand>` で、各サブコマンドの成功レスポンスが stdout に単一の YAML ドキュメントとして出力される
- `agent-exec --yaml status <missing_job_id>` のような期待される失敗でも、stdout に単一の YAML ドキュメントとして `ok: false` と `error` が出力される
- `--yaml` を付けない既定動作は従来どおり JSON のままで、既存利用例と主要統合テストが維持される
- stderr は引き続き診断ログ専用で、出力フォーマット選択によって stdout/stderr の責務が変わらない
- `README.md` と OpenSpec が JSON 既定・YAML 任意の契約を説明している

## Out of Scope

- `--output json|yaml|...` のような多値フォーマット API への一般化
- YAML 入力の受け付け
- 保存ファイル（`meta.json` / `state.json` / schema asset など）の内部表現変更
- JSON Schema 自体を YAML Schema に置き換えること
