# Design: add-yaml-output

## Context

現在の agent-exec は stdout を JSON 1 オブジェクトに固定している。

- `src/main.rs` はグローバル `verbose` を持つが、出力フォーマット選択は持たない
- `src/schema.rs` の `print_json_to_stdout` が stdout への唯一の成功/失敗レスポンス出力経路になっている
- 各サブコマンド実装は payload を組み立てて `Response::new(...).print()` を呼ぶ構造で、エラーは `main.rs` で `ErrorResponse::new(...).print()` される
- `tests/integration.rs` は stdout JSON-only 契約に強く依存している

この構造は変更点を 1 箇所へ寄せやすい一方、JSON-only 契約が仕様・README・テストに広く複製されている。

## Goals

- 既定 JSON を壊さずに YAML 出力を追加する
- 成功レスポンスと失敗レスポンスで同じフォーマット選択を適用する
- サブコマンドごとに分岐を散らさず、出力の責務を共通化したまま保つ

## Non-Goals

- 永続化 JSON ファイルの YAML 化
- 複数の追加フォーマット導入
- 既存レスポンスフィールドの意味変更

## Proposed Design

### 1. グローバル output format 選択

`Cli` にグローバル bool フラグ `yaml` を追加する。初期値は false にし、既定動作は JSON のままにする。

内部では以下のような小さな enum を持つ方針とする。

- `OutputFormat::Json`
- `OutputFormat::Yaml`

CLI 層で bool を enum に変換し、各コマンド実行関数と最終エラー出力へ明示的に渡す。これにより将来 `--output` に拡張する場合も局所変更で済む。

### 2. 共通 print 経路の format-aware 化

`src/schema.rs` の単一出力責務を維持しつつ、JSON 専用 helper を format-aware helper に置き換える。

期待する責務:
- JSON 選択時は現在と同じ 1 行 JSON を stdout へ出力する
- YAML 選択時は同一データを YAML ドキュメントとして stdout へ出力する
- どちらのフォーマットでも stderr へ余計な情報を混ぜない

`Response::print` / `ErrorResponse::print` は `OutputFormat` を引数に取る形へ変更する。コマンド側は payload 構築のみを担当し、シリアライズ詳細は `schema.rs` に閉じ込める。

### 3. エラー出力の一貫性

`main.rs` の top-level error handling も成功レスポンスと同じ `OutputFormat` を使う。これにより `job_not_found` や `internal_error` でも YAML を返せる。

### 4. schema サブコマンドの扱い

`schema` の payload 意味は維持する。

- envelope の `type: schema` は維持
- `schema_format: json-schema-draft-07` も維持
- `schema` フィールドの中身は JSON Schema を表す構造化データのまま返す

つまり `--yaml schema` は「JSON Schema を含む envelope を YAML でシリアライズしたもの」として扱う。

## Verification Strategy

統合テストでは少なくとも次を確認する。

1. 既定実行は従来どおり JSON としてパースできる
2. `--yaml` 付き成功レスポンスが YAML としてパースできる
3. `--yaml` 付き失敗レスポンスが YAML としてパースできる
4. `schema` でも YAML 出力時に `type`, `schema_format`, `schema` を保持する
5. stderr 分離が維持される

## Risks / Trade-offs

- YAML は複数行出力になりやすく、既存の「1 行 JSON」という文字列表現前提のテストは調整が必要
- YAML の暗黙型変換で値解釈がぶれないよう、Rust 側ではレスポンス構造をそのままシリアライズして文字列/真偽値/数値の意味を保つ必要がある
- 仕様上の表現は「JSON-only」から「既定 JSON、`--yaml` 時は YAML」へ更新が必要で、複数 spec delta にまたがる
