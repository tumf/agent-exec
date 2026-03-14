## MODIFIED Requirements

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys`, `cwd` を含まなければならない（MUST）。`env_keys` はキーのみを保持し、値は保存してはならない（MUST）。`cwd` はジョブ作成時の実効カレントディレクトリの絶対パスを保持しなければならない（MUST）。`cwd` の解決に失敗した場合は `null` として保存してよい（MAY）。Implementation may additionally persist the effective shell-wrapper configuration when needed to make command-string execution reproducible (MAY).

#### Scenario: shell-wrapper metadata is persisted when implementation opts in

- **WHEN** `agent-exec run` executes a command string with a non-default configured shell wrapper and `meta.json` is written
- **THEN** the required fields remain present
- **AND** any newly persisted shell-wrapper metadata reflects the effective wrapper used for that invocation
