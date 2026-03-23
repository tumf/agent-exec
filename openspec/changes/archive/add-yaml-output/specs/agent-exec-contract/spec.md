## MODIFIED Requirements

### Requirement: stdout JSON-only と stderr 分離

すべてのサブコマンドは stdout に機械可読なレスポンスを 1 つだけ出力しなければならない（MUST）。既定では JSON オブジェクト 1 つを出力しなければならない（MUST）。グローバル `--yaml` が指定された場合は、同一レスポンス内容を単一の YAML ドキュメントとして出力しなければならない（MUST）。stderr は診断ログのみに使用しなければならない（MUST）。対話的なプロンプトは行ってはならない（MUST）。

#### Scenario: status の既定標準出力
Given `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は JSON のみであり、stderr にのみログが出力される

#### Scenario: status の YAML 標準出力
Given `agent-exec --yaml status <job_id>` を実行する
When コマンドが完了する
Then stdout は単一の YAML ドキュメントである
And YAML を構造化データとして読むと `schema_version`, `ok`, `type` を含む
And stderr にのみログが出力される

### Requirement: 共通レスポンスエンベロープ

すべての成功レスポンスは `schema_version`, `ok`, `type` を含まなければならない（MUST）。`ok=false` の場合は `error` オブジェクトを含まなければならない（MUST）。この要件は JSON 既定出力と `--yaml` 出力の両方で満たされなければならない（MUST）。

#### Scenario: JSON のジョブ未検出
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok=false` を含む JSON であり、`error` が含まれる

#### Scenario: YAML のジョブ未検出
Given 存在しない `job_id` に対して `agent-exec --yaml status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok: false` を含む YAML ドキュメントである
And `error.code`, `error.message`, `error.retryable` が含まれる

### Requirement: CLI サブコマンド構成

`agent-exec` は `schema` サブコマンドを提供しなければならない（MUST）。`schema` は stdout にレスポンスエンベロープを 1 つ出力しなければならない（MUST）。既定出力では `type="schema"` の JSON を返し、`--yaml` 指定時は同一内容を YAML ドキュメントで返さなければならない（MUST）。`schema` レスポンスは `schema_format` と `schema` を含み、`schema_format` は `json-schema-draft-07` でなければならない（MUST）。

#### Scenario: schema を既定形式で取得する
Given `agent-exec schema` を実行する
When コマンドが完了する
Then stdout は `type="schema"` の JSON である
And `schema_format` は `json-schema-draft-07` である
And `schema` は JSON オブジェクトである

#### Scenario: schema を YAML 形式で取得する
Given `agent-exec --yaml schema` を実行する
When コマンドが完了する
Then stdout は単一の YAML ドキュメントである
And YAML を構造化データとして読むと `type` は `schema` である
And `schema_format` は `json-schema-draft-07` である
And `schema` はマッピングとして表現される
