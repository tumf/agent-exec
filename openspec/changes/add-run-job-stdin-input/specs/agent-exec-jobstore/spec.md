## MODIFIED Requirements

### Requirement: meta.json の必須フィールド

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys`, `cwd` を含まなければならない（MUST）。`env_keys` はキーのみを保持し、値は保存してはならない（MUST）。`cwd` はジョブ作成時の実効カレントディレクトリの絶対パスを保持しなければならない（MUST）。`cwd` の解決に失敗した場合は `null` として保存してよい（MAY）。

`create`/`start` ライフサイクルでは、`meta.json` は `start` に必要な実行定義も保持しなければならない（MUST）。これには少なくとも `inherit_env`, `env_vars`, `env_files`, `mask`, timeout 関連設定, notification 設定, shell wrapper 設定を含めなければならない（MUST）。stdin 定義が存在する場合は、`meta.json` に job directory 内で materialize 済み入力を指す `stdin_file` を保持しなければならない（MUST）。

definition-time option に由来する persisted metadata は、`create` と `run` のどちらからジョブが作られても同じフィールド構造と意味論で `meta.json` に保存されなければならない（MUST）。stdin 定義もこの共通 rule に従い、`run` と `create` は同じ `stdin_file` shape を保存しなければならない（MUST）。

#### Scenario: run と create は同じ stdin metadata shape を保存する

Given 同じ stdin 内容を使う 2 つのジョブが `agent-exec run` と `agent-exec create` でそれぞれ作成される
When 両方の `meta.json` を比較する
Then `stdin_file` は同じ意味論とフィールド shape で保存される
And 差分は即時実行の有無に限られる

#### Scenario: stdin materialization file lives in the job directory

Given stdin 定義付きジョブが作成される
When job directory を確認する
Then materialized stdin content を保持するファイルが job directory 配下に存在する
And `meta.json.stdin_file` はそのファイルを参照する
