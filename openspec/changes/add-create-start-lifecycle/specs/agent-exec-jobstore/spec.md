## MODIFIED Requirements

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `cwd` を含まなければならない（MUST）。
`create`/`start` ライフサイクルでは、`meta.json` は `start` に必要な実行定義も保持しなければならない（MUST）。これには少なくとも `inherit_env`, `env_vars`, `env_files`, `mask`, timeout 関連設定, notification 設定, shell wrapper 設定を含めなければならない（MUST）。
`env_vars` は `--env` で指定された `KEY=VALUE` を保持してよい（MAY ではなく MUST, when provided）。`env_files` は `--env-file` のファイルパスを保持しなければならない（MUST, when provided）。`cwd` はジョブ作成時の実効カレントディレクトリの絶対パスを保持しなければならない（MUST）。

#### Scenario: create が start 用の env 定義を保存する

Given `agent-exec create --cwd /tmp --env FOO=bar --env-file ./job.env -- <cmd>` を実行する
When `meta.json` が書き込まれる
Then `env_vars` に `FOO=bar` が含まれる
And `env_files` に `./job.env` が含まれる
And `cwd` は `/tmp` を絶対パスに正規化した値である

### Requirement: state.json の必須フィールド

`state.json` は少なくとも `job.id`, `job.status`, `result.exit_code`, `result.signal`, `result.duration_ms`, `updated_at` を含まなければならない（MUST）。
`job.status` は `created|running|exited|killed|failed` を表現できなければならない（MUST）。
`job.started_at` はジョブ未開始時には `null` を保持してよく、実行開始後は開始時刻を保持しなければならない（MUST）。

#### Scenario: create 直後の state

Given `agent-exec create -- <cmd>` を実行する
When `state.json` を読む
Then `job.status` は `created` である
And `job.started_at` は `null` である
And `result.exit_code`, `result.signal`, `result.duration_ms` が存在する
