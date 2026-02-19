## MODIFIED Requirements

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys`, `cwd` を含まなければならない（MUST）。`env_keys` はキーのみを保持し、値は保存してはならない（MUST）。`cwd` はジョブ作成時の実効カレントディレクトリの絶対パスを保持しなければならない（MUST）。`cwd` の解決に失敗した場合は `null` として保存してよい（MAY）。

#### Scenario: 環境変数と cwd の保存
- **WHEN** `agent-exec run --cwd /tmp --env FOO=bar -- <cmd>` を実行し `meta.json` が書き込まれる
- **THEN** `env_keys` に `FOO` が含まれ、値は保存されない
- **AND** `cwd` は `/tmp` を絶対パスに正規化した値である

#### Scenario: cwd 未指定の保存
- **WHEN** `agent-exec run -- <cmd>` を実行し `meta.json` が書き込まれる
- **THEN** `cwd` は `run` 実行プロセスの current_dir を正規化した値である
