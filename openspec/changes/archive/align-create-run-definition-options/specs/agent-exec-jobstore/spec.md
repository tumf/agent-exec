## MODIFIED Requirements

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `cwd` を含まなければならない（MUST）。
`create`/`start` ライフサイクルでは、`meta.json` は `start` に必要な実行定義も保持しなければならない（MUST）。これには少なくとも `inherit_env`, `env_vars`, `env_files`, `mask`, timeout 関連設定, notification 設定, shell wrapper 設定を含めなければならない（MUST）。

definition-time option に由来する persisted metadata は、`create` と `run` のどちらからジョブが作られても同じフィールド構造と意味論で `meta.json` に保存されなければならない（MUST）。仕様で launch-only と明示された option を除き、新しい persisted metadata field を追加する場合は `create` と `run` の両方の job creation path に反映しなければならない（MUST）。

`tags` と `notification` のような定義時メタデータは、この共通 rule の具体例として `create` と `run` の両方から同じ shape で保存されなければならない（MUST）。`create` がそれらを受け取った場合でも、保存時に notification sink を実行してはならない（MUST）。

#### Scenario: shared definition-time metadata shape

Given a persisted metadata field such as `tags` or `notification` is part of job creation
When equivalent jobs are created through `create` and `run`
Then both `meta.json` files contain the same field shape for that metadata
And any difference between the two flows is limited to execution state, not stored job definition
