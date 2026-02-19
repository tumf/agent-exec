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
