# agent-exec-jobstore Specification (Change: add-job-tags)

## MODIFIED Requirements

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys`, `cwd`, `tags` を含まなければならない（MUST）。`env_keys` はキーのみを保持し、値は保存してはならない（MUST）。`cwd` はジョブ作成時の実効カレントディレクトリの絶対パスを保持しなければならない（MUST）。`tags` は repeatable `run --tag` または `tag set --tag` で渡されたタグの重複を除いた配列でなければならず、最初に現れた順序を保持しなければならない（MUST）。`cwd` の解決に失敗した場合は `null` として保存してよい（MAY）。

#### Scenario: tags を保存する
- **WHEN** `agent-exec run --tag aaa --tag bbb -- <cmd>` を実行し `meta.json` が書き込まれる
- **THEN** `meta.json.tags` は `["aaa", "bbb"]` である

#### Scenario: 重複 tag は deduplicate される
- **WHEN** `agent-exec run --tag aaa --tag bbb --tag aaa -- <cmd>` を実行し `meta.json` が書き込まれる
- **THEN** `meta.json.tags` は `["aaa", "bbb"]` である

#### Scenario: tag set が tags のみを更新する
- **WHEN** 既存ジョブに対して `agent-exec tag set <JOB_ID> --tag aaa --tag bbb` を実行し `meta.json` が更新される
- **THEN** `meta.json.tags` は `["aaa", "bbb"]` である
- **AND** `job.id`, `command`, `created_at`, `env_keys`, `cwd` は更新前の値を保持する
