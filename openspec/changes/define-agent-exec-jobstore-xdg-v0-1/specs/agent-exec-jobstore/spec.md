# agent-exec v0.1 ジョブ保存仕様

## ADDED Requirements

### Requirement: ジョブ保存先の優先順位

ジョブ保存先は `--root` → `AGENT_EXEC_ROOT` → `$XDG_DATA_HOME/agent-exec/jobs` → 既定パスの順に解決しなければならない（MUST）。既定パスは Unix 系では `~/.local/share/agent-exec/jobs`、Windows では `BaseDirs::data_local_dir()/agent-exec/jobs` としなければならない（MUST）。

#### Scenario: XDG 未設定の Linux/macOS
Given `--root` と `AGENT_EXEC_ROOT` と `XDG_DATA_HOME` が未設定である
When `agent-exec run -- <cmd>` を実行する
Then ジョブは `~/.local/share/agent-exec/jobs/<job_id>` に作成される

#### Scenario: Windows の既定パス
Given Windows 環境で `--root` と `AGENT_EXEC_ROOT` と `XDG_DATA_HOME` が未設定である
When `agent-exec run -- <cmd>` を実行する
Then ジョブは `BaseDirs::data_local_dir()/agent-exec/jobs/<job_id>` に作成される

### Requirement: ジョブディレクトリ構造

各ジョブは `<root>/<job_id>/` に作成し、`meta.json`, `state.json`, `stdout.log`, `stderr.log`, `full.log` を含まなければならない（MUST）。

#### Scenario: ジョブディレクトリの作成
Given `agent-exec run -- <cmd>` を実行する
When ジョブが作成される
Then ジョブディレクトリに `meta.json` と `state.json` と `stdout.log` と `stderr.log` と `full.log` が存在する

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys` を含まなければならない（MUST）。`env_keys` はキーのみを保持し、値は保存してはならない（MUST）。

#### Scenario: 環境変数の保存
Given `agent-exec run --env FOO=bar -- <cmd>` を実行する
When `meta.json` が書き込まれる
Then `env_keys` に `FOO` が含まれ、値は保存されない

### Requirement: state.json の必須フィールド

`state.json` は少なくとも `job.id`, `job.status`, `job.started_at`, `result.exit_code`, `result.signal`, `result.duration_ms`, `updated_at` を含まなければならない（MUST）。

#### Scenario: 実行中の state
Given 実行中のジョブが存在する
When `state.json` を読む
Then `job.status` が `running` であり、`updated_at` が含まれる

### Requirement: 原子的な書き込み

`meta.json` と `state.json` は一時ファイルへ書き込んだ後にリネームすることで原子的に更新しなければならない（MUST）。

#### Scenario: state.json の更新
Given 実行中のジョブがある
When `state.json` が更新される
Then 途中で破損した JSON が観測されない
