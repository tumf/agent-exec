# agent-exec-contract Specification

## Purpose
TBD - created by archiving change define-agent-exec-contract-v0-1. Update Purpose after archive.
## Requirements
### Requirement: CLI サブコマンド構成

`agent-exec` は `schema` サブコマンドを提供しなければならない（MUST）。`schema` は stdout に `type="schema"` の JSON を 1 つ出力しなければならない（MUST）。`schema` の JSON は `schema_format` と `schema` を含み、`schema_format` は `json-schema-draft-07` でなければならない（MUST）。

#### Scenario: schema を取得する

Given `agent-exec schema` を実行する
When コマンドが完了する
Then stdout は `type="schema"` の JSON である
And `schema_format` は `json-schema-draft-07` である
And `schema` は JSON オブジェクトである

### Requirement: ヘルプは英語

`-h`/`--help` は常に有効でなければならない（MUST）。トップレベルおよび各サブコマンドのヘルプ文言は英語でなければならない（MUST）。

#### Scenario: サブコマンドヘルプ
Given `agent-exec run --help` を実行する
When ヘルプが表示される
Then 表示内容は英語である

### Requirement: stdout JSON-only と stderr 分離

すべてのサブコマンドは stdout に JSON オブジェクト 1 つのみを出力しなければならない（MUST）。stderr は診断ログのみに使用しなければならない（MUST）。対話的なプロンプトは行ってはならない（MUST）。

#### Scenario: status の標準出力
Given `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は JSON のみであり、stderr にのみログが出力される

### Requirement: 共通レスポンスエンベロープ

すべての出力 JSON は `schema_version`, `ok`, `type` を含まなければならない（MUST）。`ok=false` の場合は `error` オブジェクトを含まなければならない（MUST）。

#### Scenario: ジョブ未検出
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok=false` を含む JSON であり、`error` が含まれる

### Requirement: エラーオブジェクト形式

`error` は `code`, `message`, `retryable` を必須フィールドとして持たなければならない（MUST）。

#### Scenario: エラー応答の必須フィールド
Given `agent-exec status <missing_job_id>` を実行する
When コマンドが完了する
Then `error.code` と `error.message` と `error.retryable` が含まれる

### Requirement: 終了コード

成功時は `0`、期待される失敗（対象未検出/バリデーション失敗/I/O など）は `1`、CLI usage エラーは `2` を返さなければならない（MUST）。

#### Scenario: 期待される失敗の終了コード
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが終了する
Then 終了コードは `1` である


#

## Requirements

### Requirement: schema_version のバージョニングポリシー

`schema_version` は `"MAJOR.MINOR"` 形式の文字列でなければならない（MUST）。両セグメントは非負整数であり、先頭ゼロを含んではならない（MUST NOT）。

後方互換のあるフィールド追加（optional field の追加、enum variant の追加）は MINOR bump で行わなければならない（MUST）。既存フィールドの削除、型変更、意味変更、required 化は MAJOR bump を要する（MUST）。

`schema_version` が bump されるとき、リポジトリ直下の `CHANGELOG.md` に対応する `## schema <version>` セクションを追加しなければならない（MUST）。

クライアント／エージェントは MAJOR が一致する JSON を解釈できなければならない（MUST）。未知の optional field を受け取った場合はそれを無視できなければならない（forward compatibility、MUST）。MAJOR 不一致の場合はエラー扱いとしてよい（MAY）。

#### Scenario: adding an optional field bumps MINOR

**Given**: canonical `schema_version = "0.1"`
**When**: a new optional field is added to `RunData`
**Then**: the next `schema_version` is `"0.2"` with a `## schema 0.2` entry in CHANGELOG.md

#### Scenario: removing a field bumps MAJOR

**Given**: canonical `schema_version = "0.9"`
**When**: an existing field is removed from `RunData`
**Then**: the next `schema_version` is `"1.0"` with a `## schema 1.0` entry in CHANGELOG.md
