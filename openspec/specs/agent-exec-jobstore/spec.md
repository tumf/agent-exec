# agent-exec-jobstore Specification

## Purpose
TBD - created by archiving change define-agent-exec-jobstore-xdg-v0-1. Update Purpose after archive.
## Requirements
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

`meta.json` は少なくとも `job.id`, `command`, `created_at`, `env_keys`, `cwd` を含まなければならない（MUST）。`env_keys` はキーのみを保持し、値は保存してはならない（MUST）。`cwd` はジョブ作成時の実効カレントディレクトリの絶対パスを保持しなければならない（MUST）。`cwd` の解決に失敗した場合は `null` として保存してよい（MAY）。

`create`/`start` ライフサイクルでは、`meta.json` は `start` に必要な実行定義も保持しなければならない（MUST）。これには少なくとも `inherit_env`, `env_vars`, `env_files`, `mask`, timeout 関連設定, notification 設定, shell wrapper 設定を含めなければならない（MUST）。

definition-time option に由来する persisted metadata は、`create` と `run` のどちらからジョブが作られても同じフィールド構造と意味論で `meta.json` に保存されなければならない（MUST）。仕様で launch-only と明示された option を除き、新しい persisted metadata field を追加する場合は `create` と `run` の両方の job creation path に反映しなければならない（MUST）。

`tags` と `notification` のような定義時メタデータは、この共通 rule の具体例として `create` と `run` の両方から同じ shape で保存されなければならない（MUST）。`create` がそれらを受け取った場合でも、保存時に notification sink を実行してはならない（MUST）。

#### Scenario: shared definition-time metadata shape

Given a persisted metadata field such as `tags` or `notification` is part of job creation
When equivalent jobs are created through `create` and `run`
Then both `meta.json` files contain the same field shape for that metadata
And any difference between the two flows is limited to execution state, not stored job definition

#### Scenario: 環境変数と cwd の保存
- **WHEN** `agent-exec run --cwd /tmp --env FOO=bar -- <cmd>` を実行し `meta.json` が書き込まれる
- **THEN** `env_keys` に `FOO` が含まれ、値は保存されない
- **AND** `cwd` は `/tmp` を絶対パスに正規化した値である

#### Scenario: cwd 未指定の保存
- **WHEN** `agent-exec run -- <cmd>` を実行し `meta.json` が書き込まれる
- **THEN** `cwd` は `run` 実行プロセスの current_dir を正規化した値である

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


#


#


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

## Requirements

### Requirement: stdin.bin の保存仕様

`--stdin <VALUE>` / `--stdin -` / `--stdin-file <PATH>` によって materialize される入力は、job directory 直下のファイル `stdin.bin`（相対パス固定）として保存しなければならない（MUST）。`meta.json.stdin_file` はこの相対ファイル名 `"stdin.bin"` を保持しなければならない（MUST）。

Unix 系プラットフォームでは `stdin.bin` のパーミッションは `0o600` で作成しなければならない（MUST）。umask の影響を受けてはならない（MUST NOT）。Windows では NTFS ACL の既定を維持する（owner のみアクセス可能）。

書き込み時の入力サイズは既定 64 MiB（67108864 bytes）を上限としなければならない（MUST）。`--stdin-max-bytes <N>` で上限を明示指定できなければならない（MUST）。上限超過時は起動前に `error.code="stdin_too_large"` で失敗しなければならない（MUST）。

#### Scenario: stdin.bin is created with 0o600 on Unix

**Given**: a Unix-like platform
**When**: `agent-exec create --stdin "secret" -- cat` is executed
**Then**: `stdin.bin` exists inside the job directory
**And**: the file mode is `0o600`

#### Scenario: oversized stdin fails with stdin_too_large

**Given**: a 65 MiB input via `--stdin-file ./big.bin`
**When**: `agent-exec run --stdin-file ./big.bin -- cat` is executed with default `--stdin-max-bytes`
**Then**: the command fails with `error.code="stdin_too_large"` before launching the workload

## Requirements

### Requirement: job_id の生成仕様

新規生成する `job_id` は 32 文字の小文字 16 進数文字列でなければならない（MUST）。エントロピー源は OS CSPRNG（128 bit 以上）でなければならない（MUST）。`short_job_id` はこの `job_id` の先頭 7 文字でなければならない（MUST）。

衝突検出（同名ディレクトリが既に存在する）時は最大 16 回まで再生成を試行しなければならない（MUST）。16 回連続で衝突した場合は `error.code="io_error"` の構造化エラーを返さなければならない（MUST）。無制限 loop をしてはならない（MUST NOT）。

#### Scenario: generated job_id is 32-char lowercase hex

**Given**: `agent-exec run -- echo hi` is executed
**When**: the JSON response is returned
**Then**: `job_id` matches `^[0-9a-f]{32}$`

#### Scenario: 16 consecutive collisions return io_error

**Given**: a fake RNG produces the same 16 bytes on 16 consecutive draws
**And**: a directory with that `job_id` already exists
**When**: `generate_job_id` is called
**Then**: the call returns an error mapped to `error.code="io_error"`
