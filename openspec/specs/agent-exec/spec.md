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

### Requirement: run/start は既定で inline output を返す

`run` と `start` は既定で最大 10 秒待機し、待機中に観測できた stdout/stderr を inline で返さなければならない（MUST）。
`--no-wait` 指定時は待機せず即時返却しなければならない（MUST）。
レスポンスには `waited_ms`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を含めなければならない（MUST）。

#### Scenario: run は既定で inline output を返す

Given `agent-exec run -- <cmd>` を実行する
When コマンドが返る
Then レスポンスに `job_id` と `state` とログパスが含まれる
And `waited_ms` と `stdout`/`stderr` が含まれる
And `stdout_range`/`stderr_range` と `stdout_total_bytes`/`stderr_total_bytes` が含まれる

### Requirement: head/tail の UTF-8 lossy + range 契約

`run`/`start` の `stdout`/`stderr` はログ先頭（head）を UTF-8 lossy で返さなければならない（MUST）。
`tail` の `stdout`/`stderr` はログ末尾（tail）を UTF-8 lossy で返さなければならない（MUST）。
いずれも `encoding="utf-8-lossy"` と raw byte range `[begin, end)` を返さなければならない（MUST）。

#### Scenario: 非 UTF-8 バイトを含むログ
Given `stdout.log` に非 UTF-8 バイト列が含まれる
When `agent-exec tail <job_id>` を実行する
Then stdout の JSON には `encoding="utf-8-lossy"` が含まれる
And `stdout_range` が含まれる

### Requirement: Windows の kill 対応

Windows では `kill` がプロセスツリーを終了させなければならない（MUST）。`--signal` は `TERM|INT|KILL` を受け付け、未対応のシグナルは `KILL` 相当で扱わなければならない（MUST）。

#### Scenario: Windows の kill 実行
Given Windows 環境で `agent-exec kill <job_id> --signal TERM` を実行する
When コマンドが成功する
Then JSON の `ok=true` が返り、対象ジョブのプロセスツリーが終了する

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


### Requirement: run のジョブ生成と inline output

`run` はジョブを起動し、既定で最大 10 秒待機して inline output を返さなければならない（MUST）。
`--no-wait` が指定された場合は即時返却しなければならない（MUST）。
`--wait`、`--until`、`--forever`、`--no-wait`、`--max-bytes` を受け付けなければならない（MUST）。
`run`/`create`/`_supervise` の runtime 制御時間オプション（`--timeout`、`--kill-after`、`--progress-every`）は人間向け契約として秒単位で提示されなければならない（MUST）。

#### Scenario: run は既定待機で inline output を返す

Given `agent-exec run -- sh -c "sleep 1; echo hi"` を実行する
When `run` の JSON が返る
Then `job_id` が含まれる
And `waited_ms` と `stdout` が含まれる
And `stdout_range[0]` は `0` である

#### Scenario: run --no-wait は即時返却する

Given `agent-exec run --no-wait -- sh -c "sleep 1; echo hi"` を実行する
When `run` の JSON が返る
Then コマンドは追加待機せず返る
And `waited_ms` は短時間である


### Requirement: run のジョブ生成と初回 inline output

`run` はジョブを起動し、既定では bare `--wait`（`--wait true` と同義）と `--until 10` 相当の待機予算内で観測できた stdout / stderr を初回レスポンスに含めなければならない（MUST）。`--no-wait` は `--wait false --until 0` のエイリアスであり、追加待機なしの launch-only 返却を明示的に選べなければならない（MUST）。

`run` の出力は top-level の `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` で表現しなければならない（MUST）。range は raw byte offset の `[begin, end]` 配列で、意味は half-open interval `[begin, end)` とする（MUST）。

#### Scenario: run 既定は最大 10 秒待機して head を返す

Given `agent-exec run -- sh -c "printf 'hello'"` を実行する
When `run` の JSON が返る
Then `state` は終端状態である
And `stdout` は `hello` を含む
And `stdout_range` は `[0, 5]` である
And `stdout_total_bytes` は `5` である

#### Scenario: run --no-wait は待機なしで返る

Given `agent-exec run --no-wait -- sh -c "sleep 60"` を実行する
When `run` の JSON が返る
Then `waited_ms` は 0 近傍である
And ジョブは継続実行してよい

### Requirement: tail は range 付き末尾観測 API

`tail` はログ末尾の観測を担い、`stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を返さなければならない（MUST）。`tail` の range は返却した末尾部分の raw byte 区間を示さなければならない（MUST）。

#### Scenario: tail は末尾の range を返す

Given stdout.log が 1000 bytes あり、最後の 120 bytes が取得対象である
When `agent-exec tail <job_id>` を実行する
Then `stdout_range` は `[880, 1000]` である
And `stdout_total_bytes` は `1000` である

### Requirement: run/start/tail は range 契約を共有する

`run`, `start`, `tail` が返す stdout / stderr 本文は、同じ field 名と range 契約を共有しなければならない（MUST）。`snapshot`, `final_snapshot`, `truncated`, `stdout_tail`, `stderr_tail`, `stdout_observed_bytes`, `stderr_observed_bytes`, `stdout_included_bytes`, `stderr_included_bytes` を canonical field 名として返してはならない（MUST NOT）。

#### Scenario: canonical output fields are unified

Given `run`, `start`, `tail` の各レスポンスを比較する
When 本文と byte 範囲フィールドを確認する
Then 3 つとも `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を使う
And 削除済み field 名は含まれない

## Requirements

### Requirement: hash-like job IDs for new jobs

`run`, `create`, および `POST /exec` が新規 job を作成する際の canonical `job_id` は、ULID ではなく固定長の小文字 hex ランダム識別子でなければならない（MUST）。新規 `job_id` は同一 root 配下の既存 job directory 名と衝突してはならず、衝突時は再生成しなければならない（MUST）。

#### Scenario: run creates a hash-like job ID

Given `agent-exec run -- echo hi` を実行する
When ジョブが作成される
Then 返却される `job_id` は `[0-9a-f]` のみで構成される固定長文字列である
And `job_id` は `01` 始まりの ULID 形式を前提にしていない
And `<root>/<job_id>/` が作成される

#### Scenario: create creates a hash-like job ID

Given `agent-exec create -- echo hi` を実行する
When ジョブが作成される
Then 返却される `job_id` は `[0-9a-f]` のみで構成される固定長文字列である
And `meta.json.job.id` はその完全 ID と一致する

### Requirement: short job ID in list output

`list` の各 job summary は完全な `job_id` に加えて、完全 ID の先頭 7 文字を表す `short_job_id` を含まなければならない（MUST）。`short_job_id` は人間向け表示用の省略表現であり、canonical identifier の代替ではない（MUST）。

#### Scenario: list returns short job IDs

Given 少なくとも 1 件の job が存在する
When `agent-exec list` を実行する
Then `jobs` の各要素は `job_id` と `short_job_id` を含む
And `short_job_id` は `job_id` の先頭 7 文字と一致する


### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `short_job_id`, `state`, `started_at` を含み、`exit_code` と `finished_at` と `updated_at` は存在する場合にのみ含めてよい（MAY）。

#### Scenario: list が必須フィールドを返す

Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `short_job_id`, `state`, `started_at` を含む

### Requirement: create and start lifecycle commands

`agent-exec` MUST support a two-step lifecycle in addition to immediate `run`. `create` MUST persist a job definition without launching the command, and `start <job_id>` MUST launch a previously created job using the persisted definition. job lookup は完全一致または一意な先頭 prefix による指定を受け付けなければならない（MUST）。完全一致が存在しない場合に prefix 一致候補が 1 件だけなら、その job を解決しなければならない（MUST）。prefix 一致候補が複数ある場合は `ambiguous_job_id` を返さなければならない（MUST）。

#### Scenario: unique prefix resolves a hash-like job

Given hash-like `job_id` を持つ job が存在する
And その先頭 prefix が root 配下で一意である
When `agent-exec status <prefix>` を実行する
Then コマンドは対応する完全 `job_id` の job を解決して成功する

#### Scenario: ambiguous prefix is rejected

Given 2 件の job が同じ先頭 prefix を共有して存在する
When `agent-exec status <shared-prefix>` を実行する
Then コマンドは `ambiguous_job_id` を返して失敗する

### Requirement: backward compatibility for existing jobs

job lookup は既存の ULID 形式 job directory と新しい hash-like job directory の両方を扱えなければならない（MUST）。新規生成は新形式へ移行してよいが、既存 job の読み取り・待機・停止・削除・開始互換は維持しなければならない（MUST）。

#### Scenario: existing ULID jobs remain addressable

Given root 配下に既存 ULID 形式の job directory が存在する
When `agent-exec status <ulid-job-id>` を実行する
Then コマンドはその job を解決して状態を返す
And 新形式 job の導入によって既存 ULID job の参照は壊れない


### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `short_job_id`, `state`, `started_at` を含まなければならない（MUST）。

state.json が読める場合、各エントリは `updated_at` を必ず含めなければならない（MUST）。ジョブが終端状態（succeeded / failed / killed / timeout）の場合、`finished_at` と `exit_code` を必ず含めなければならない（MUST）。state.json がレース条件で未作成・破損している場合に限り、これらは省略してよい（MAY）。

#### Scenario: list が必須フィールドを返す

Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `short_job_id`, `state`, `started_at` を含む

#### Scenario: list includes progress for running jobs

**Given**: a running job whose state.json is readable
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`
**And**: `finished_at` and `exit_code` are absent

#### Scenario: list includes terminal fields for finished jobs

**Given**: a finished job
**When**: `agent-exec list` is executed
**Then**: the job entry includes `updated_at`, `finished_at`, and `exit_code`

### Requirement: list の件数制限と truncated フラグ

`list` の `--limit <N>` は返却する件数の上限を指定し、既定値は `50` でなければならない（MUST）。`--limit 0` は「明示的無制限」を意味し受理しなければならない（MUST）。

レスポンスには `truncated: bool` を必ず含めなければならない（MUST）。制限に達し未返却のジョブが残っている場合 `truncated=true`、それ以外は `false` でなければならない（MUST）。

#### Scenario: list default returns up to 50 jobs with truncated=true

**Given**: 60 jobs exist under the caller's cwd
**When**: `agent-exec list` is executed
**Then**: `jobs` has length `50`
**And**: `truncated` is `true`

#### Scenario: list --limit 0 returns all jobs

**Given**: 60 jobs exist under the caller's cwd
**When**: `agent-exec list --limit 0` is executed
**Then**: `jobs` has length `60`
**And**: `truncated` is `false`
