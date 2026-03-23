## MODIFIED Requirements

### Requirement: JSON-only stdout

すべてのサブコマンドは stdout に機械可読なレスポンスを 1 つのみ出力しなければならない（MUST）。既定では JSON オブジェクト 1 つを出力しなければならない（MUST）。グローバル `--yaml` 指定時は同一レスポンス内容を YAML ドキュメント 1 つとして出力しなければならない（MUST）。`--help`/`--version` と clap の usage エラーのみ例外とする。stderr は診断ログ専用としなければならない（MUST）。

#### Scenario: status の既定標準出力
Given `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は JSON のみで、stderr には任意の診断ログが出力される

#### Scenario: status の YAML 標準出力
Given `agent-exec --yaml status <job_id>` を実行する
When コマンドが完了する
Then stdout は YAML ドキュメント 1 つである
And YAML を構造化データとして読むと `schema_version`, `ok`, `type` を含む

### Requirement: 共通 JSON スキーマ

すべての成功レスポンスは共通フィールド `schema_version`, `ok`, `type` を持たなければならない（MUST）。`ok=false` の場合は必ず `error` を含まなければならない（MUST）。この共通構造は既定 JSON と `--yaml` の両方で保たれなければならない（MUST）。

#### Scenario: JSON のジョブ未検出
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok=false` を含む JSON であり、`error.code` が `job_not_found` である

#### Scenario: YAML のジョブ未検出
Given 存在しない `job_id` に対して `agent-exec --yaml status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok: false` を含む YAML ドキュメントである
And `error.code` が `job_not_found` である

### Requirement: README の利用導線

README は `run/status/tail/wait/kill/list` を対象にしたコピペ可能な使用例を含めなければならない（MUST）。README は stdout が既定で JSON であり、`--yaml` 指定時は YAML を返すこと、stderr が診断ログであることを明記しなければならない（MUST）。

#### Scenario: README のコマンド例

Given リポジトリの `README.md` を読む
When 利用例セクションを確認する
Then `run`/`status`/`tail`/`wait`/`kill`/`list` の例が含まれる
And stdout の既定が JSON である旨が明記されている
And `--yaml` で YAML を返せる旨が明記されている
