## MODIFIED Requirements

### Requirement: ジョブディレクトリ構造

各ジョブは `<root>/<job_id>/` に作成し、`meta.json`, `state.json`, `stdout.log`, `stderr.log`, `full.log` を含まなければならない（MUST）。新規 job に対する `job_id` directory 名は小文字 hex ベースの hash-like ID でなければならない（MUST）。既存 ULID directory は互換のため引き続き開けなければならない（MUST）。

#### Scenario: new jobs use hash-like directory names

Given `agent-exec run -- <cmd>` を実行する
When ジョブが作成される
Then job directory 名は返却された完全 `job_id` と一致する
And その directory 名は `[0-9a-f]` のみで構成される固定長文字列である

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys`, `cwd` を含まなければならない（MUST）。`job.id` は job directory 名と一致する完全な canonical ID を保持しなければならない（MUST）。一覧や UI 向けの短縮表示は `meta.json` の canonical ID を置き換えてはならない（MUST NOT）。

#### Scenario: meta.json keeps the full canonical ID

Given 新形式 job が作成される
When `meta.json` を読む
Then `job.id` は short 表示ではなく完全 `job_id` と一致する
And 短縮表示の都合で canonical ID が切り詰められて保存されることはない
