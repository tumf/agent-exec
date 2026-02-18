# agent-exec Specification

## Purpose
TBD - created by archiving change define-agent-exec-v0-1. Update Purpose after archive.
## Requirements
### Requirement: JSON-only stdout

すべてのサブコマンドは stdout に JSON オブジェクト 1 つのみを出力しなければならない（MUST）。`--help`/`--version` と clap の usage エラーのみ例外とする。stderr は診断ログ専用としなければならない（MUST）。

#### Scenario: status の標準出力
Given `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は JSON のみで、stderr には任意の診断ログが出力される

### Requirement: 共通 JSON スキーマ

すべての出力 JSON は共通フィールド `schema_version`, `ok`, `type` を持たなければならない（MUST）。`ok=false` の場合は必ず `error` を含まなければならない（MUST）。

#### Scenario: ジョブ未検出
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok=false` を含む JSON であり、`error.code` が `job_not_found` である

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

### Requirement: run のジョブ生成とスナップショット

`run` は `snapshot-after` の待機時間を最大 10,000ms に制限しなければならない（MUST）。
`run` の `snapshot` は `stdout_observed_bytes`/`stderr_observed_bytes` と
`stdout_included_bytes`/`stderr_included_bytes` を含めなければならない（MUST）。

#### Scenario: snapshot の bytes メトリクス

Given `agent-exec run --snapshot-after 500 --max-bytes 64 -- <cmd>` を実行する
When snapshot が返る
Then `snapshot.stdout_observed_bytes` と `snapshot.stderr_observed_bytes` が含まれる
And `snapshot.stdout_included_bytes` と `snapshot.stderr_included_bytes` が含まれる

### Requirement: tail/snapshot の UTF-8 lossy

`tail` および `run` の `snapshot` はログ末尾を UTF-8 lossy で文字列化し、`encoding="utf-8-lossy"` を返さなければならない（MUST）。

#### Scenario: 非 UTF-8 バイトを含むログ
Given `stdout.log` に非 UTF-8 バイト列が含まれる
When `agent-exec tail <job_id>` を実行する
Then stdout の JSON には `encoding="utf-8-lossy"` が含まれる

### Requirement: Windows の kill 対応

Windows では `kill` がプロセスツリーを終了させなければならない（MUST）。`--signal` は `TERM|INT|KILL` を受け付け、未対応のシグナルは `KILL` 相当で扱わなければならない（MUST）。

#### Scenario: Windows の kill 実行
Given Windows 環境で `agent-exec kill <job_id> --signal TERM` を実行する
When コマンドが成功する
Then JSON の `ok=true` が返り、対象ジョブのプロセスツリーが終了する

### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `state`, `started_at` を含み、`exit_code` と `finished_at` と `updated_at` は存在する場合にのみ含めてよい（MAY）。

#### Scenario: list が必須フィールドを返す
Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `state`, `started_at` を含む

### Requirement: list の並び順と制約

`list` は `started_at` の降順で `jobs` を返さなければならない（MUST）。`--limit` が指定された場合は上限件数まで返し、超過した場合は `truncated=true` を返さなければならない（MUST）。

#### Scenario: limit による切り詰め
Given `agent-exec list --limit 2` を実行する
When ジョブが 3 件以上存在する
Then `jobs` の長さは 2 である
And `truncated` は `true` である

### Requirement: root 不在時の挙動

root が存在しない場合、`list` はエラーではなく `jobs=[]` を返さなければならない（MUST）。

#### Scenario: root が存在しない
Given `agent-exec list --root /path/does/not/exist` を実行する
When コマンドが完了する
Then `jobs` は空配列である

