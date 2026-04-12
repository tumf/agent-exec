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

### Requirement: run/start は起動メタデータを即時返却する

`run` と `start` はジョブ起動を主責務とし、返却前に snapshot 取得待機を行ってはならない（MUST NOT）。
起動レスポンスは `job_id`、初期 `state`、ログパスなどの起動メタデータに限定しなければならない（MUST）。
完了待機は `wait`、出力観測は `tail` に分離しなければならない（MUST）。

#### Scenario: run は即時に起動メタデータを返す

Given `agent-exec run -- <cmd>` を実行する
When コマンドが返る
Then レスポンスに `job_id` と `state` とログパスが含まれる
And `snapshot` / `final_snapshot` / `waited_ms` は含まれない

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

### Requirement: list の状態フィルタ

`list` は `--state <state>` を受け付け、指定時は `jobs` を `jobs[].state == <state>` に一致するものだけ返さなければならない（MUST）。
`state` の値は `running|exited|killed|failed|unknown` に限定され、未知の値は usage エラーとする（MUST）。
`--state` 指定時はフィルタ適用後の件数に対して `--limit` を適用し、必要に応じて `truncated=true` としなければならない（MUST）。

#### Scenario: 実行中ジョブのみの取得
Given 実行中ジョブと終了済みジョブが存在する
When `agent-exec list --state running` を実行する
Then `jobs` は `state=running` のジョブのみを含む
And `jobs` の全要素で `state` は `running` である

### Requirement: list の cwd フィルタ

`list` は `meta.json.cwd` が対象ディレクトリと一致するジョブのみを返さなければならない（MUST）。既定の対象ディレクトリは `list` 実行プロセスの current_dir とする（MUST）。`--cwd <PATH>` が指定された場合は、そのパスを対象ディレクトリとして扱わなければならない（MUST）。`--all` が指定された場合は cwd フィルタを無効化し、対象ディレクトリ一致の条件を適用してはならない（MUST）。対象ディレクトリと `meta.json.cwd` は同一の正規化規則（可能なら `canonicalize`、失敗時は絶対化）で比較しなければならない（MUST）。

#### Scenario: デフォルトの current_dir 一致
- **WHEN** current_dir が `A` の状態で `agent-exec list` を実行する
- **THEN** `jobs` は `meta.json.cwd == A` のジョブのみを含む

#### Scenario: --cwd 指定のフィルタ
- **WHEN** current_dir が `B` の状態で `agent-exec list --cwd A` を実行する
- **THEN** `jobs` は `meta.json.cwd == A` のジョブのみを含む

#### Scenario: --all による全件表示
- **WHEN** current_dir が `B` の状態で `agent-exec list --all` を実行する
- **THEN** `jobs` は cwd 一致条件で除外されない

### Requirement: list の --all/--cwd 排他

`list` は `--all` と `--cwd` の同時指定を受け付けてはならず、usage エラーとして終了しなければならない（MUST）。

#### Scenario: --all と --cwd の同時指定
- **WHEN** `agent-exec list --all --cwd /tmp` を実行する
- **THEN** コマンドは usage エラーとして終了コード 2 を返す

### Requirement: create and start lifecycle commands

`agent-exec` MUST support a two-step lifecycle in addition to immediate `run`. `create` MUST persist a job definition without launching the command, and `start <job_id>` MUST launch a previously created job using the persisted definition.

For the job-definition portion of the lifecycle, `create` and `run` MUST accept the same definition-time options whenever those options contribute to persisted job metadata (MUST). `run` MAY additionally accept immediate-execution or observation-time options that `create` does not expose (MAY). `start` MUST consume the persisted definition rather than redefining those definition-time options (MUST).

This shared definition-time option surface MUST include persisted tags and persisted notification settings when those metadata families are supported (MUST). `create` MUST save those values without launching notification side effects, and `start` MUST use the saved values when launching the job (MUST).

#### Scenario: run and create share persisted definition inputs

Given a definition-time option contributes to `meta.json`
When that option is supported for `agent-exec run`
Then `agent-exec create` also accepts it unless the spec explicitly documents it as launch-only
And jobs created via `create` and via `run` persist the same metadata shape for that option

#### Scenario: create stores tags and notifications as shared definition metadata

Given `agent-exec create --tag aaa --notify-command 'cat >/tmp/event.json' -- sh -c "echo hi"` is executed
When the command returns
Then the job metadata stores tag `aaa` and the configured notification settings
And no notification command has been executed during `create`

### Requirement: README の利用導線

README は `run/status/tail/wait/kill/list` を対象にしたコピペ可能な使用例を含めなければならない（MUST）。README は stdout が JSON-only であり、stderr が診断ログであることを明記しなければならない（MUST）。

#### Scenario: README のコマンド例

Given リポジトリの `README.md` を読む
When 利用例セクションを確認する
Then `run`/`status`/`tail`/`wait`/`kill`/`list` の例が含まれる
And stdout が JSON-only である旨が明記されている


#


#


#


#


#
