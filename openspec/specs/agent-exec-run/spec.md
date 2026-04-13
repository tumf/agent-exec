# agent-exec-run Specification

## Purpose
TBD - created by archiving change define-agent-exec-run-supervise-v0-1. Update Purpose after archive.
## Requirements
### Requirement: run の監視分離

Issue `#5` verification must distinguish between visible success output and actual workload termination. A job must not be considered reliably complete merely because its logs contain apparent success lines, and regressions for lingering `running` state must include a reproduction shape where the wrapped workload process itself may remain alive after success-like output (MUST).

#### Scenario: cflx-like workload logs success before job leaves running

Given a workload launched via `agent-exec run -- <workload>` emits success-like completion lines to stdout
And the job still has a live wrapped workload process after those lines are visible
When `agent-exec status <job_id>` and `agent-exec wait <job_id>` are evaluated for issue `#5`
Then the regression analysis must treat this as a distinct failure shape from descendant-held stdio only
And any accepted fix must be verified against this workload-liveness case, not only shell-only synthetic cases

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

#### Scenario: tail が末尾観測 API である
Given 実行中または完了済みのジョブが存在する
When `agent-exec tail --tail-lines 10 --max-bytes 1024 <job_id>` を実行する
Then `stdout`/`stderr` と `encoding="utf-8-lossy"` が返る
And `stdout_range`/`stderr_range` が返る

### Requirement: ログファイル

`stdout.log` と `stderr.log` はそれぞれのストリームを追記保存しなければならない（MUST）。`full.log` は時刻とストリーム種別を含む 1 行形式で追記しなければならない（MUST）。

#### Scenario: full.log の形式
Given 実行中のジョブがある
When `full.log` が追記される
Then 各行は `RFC3339 timestamp` と `[STDOUT]` または `[STDERR]` を含む

### Requirement: timeout と kill-after

`--timeout` が指定された場合、期限到達時に終了シグナルを送信し、`--kill-after` 経過後も生存している場合は強制終了しなければならない（MUST）。

#### Scenario: timeout の強制終了
Given `agent-exec run --timeout 1s --kill-after 1s -- <cmd>` を実行する
When 2 秒経過する
Then 対象プロセスは終了している

### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。

`run` が受け付ける definition-time option は、同じ persisted job definition を表す限り `create` でも受け付けなければならない（MUST）。そのような option は `run` と `create` の両方で同じ `meta.json` 意味論に落ちるよう定義しなければならない（MUST）。`run --wait` は現行の正規観測オプションであり、`--no-wait`/`--until`/`--forever` と整合した意味で提供されなければならない（MUST）。

#### Scenario: persisted env definition stays aligned between create and run

Given `--env-file A --env KEY=VALUE` is part of the persisted job definition
When a job is created via `agent-exec create` and another equivalent job is created via `agent-exec run`
Then both jobs persist equivalent environment-definition metadata
And any difference between the commands is limited to immediate execution behavior

### Requirement: create initial tag assignment

`create` must accept repeatable `--tag <TAG>` using the same validation and deduplication rules as `run` (MUST). The persisted tags must be available to `start` without requiring any additional tag mutation command (MUST).

#### Scenario: create stores deduplicated tags

Given `agent-exec create --tag aaa --tag bbb --tag aaa -- sh -c "echo hi"` is executed
When the job metadata is written
Then the persisted tags are `["aaa", "bbb"]`
And a later `agent-exec start <job_id>` uses those tags as the job's initial tag set

### Requirement: run completion notification configuration

`run` must support persisted notification sinks for both job completion and output matches (MUST). Completion delivery must continue to consult the latest persisted notification metadata at dispatch time rather than assuming launch-time values are still current (MUST). When output-match notification metadata is present, the running supervisor must consult the latest persisted settings for newly observed stdout/stderr lines and emit `job.output.matched` events for matching future lines (MUST).

Notification settings are definition-time metadata and therefore must be accepted by both `create` and `run` (MUST). Jobs defined through either path must persist the same notification metadata shape before execution begins (MUST).

#### Scenario: create and run persist the same notification metadata

Given `--notify-command 'cat >/tmp/event.json' --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json'` is provided as job-definition input
When one job is defined with `agent-exec create` and another with `agent-exec run`
Then both jobs persist equivalent notification metadata
And only the `run` path begins execution immediately

#### Scenario: create persists output-match notifications for later start

Given `agent-exec create --output-pattern 'ERROR' --output-command 'cat >/tmp/output.json' -- sh -c "echo ERROR"` is executed
When `agent-exec start <job_id>` later launches that created job
Then the running job uses the output-match notification settings saved during `create`
And `create` itself did not trigger any notification delivery

#### Scenario: env の上書き
Given `--env-file A --env-file B --env KEY=VALUE` を指定する
When 環境が構築される
Then `KEY` は `--env` の値で上書きされる

### Requirement: mask の適用範囲

`--mask KEY` は JSON 出力および `meta.json` の表示にのみ適用され、実際のプロセス環境は変更してはならない（MUST）。

#### Scenario: mask の表示
Given `--env SECRET=aaa --mask SECRET` を指定する
When `run` の JSON が返る
Then `SECRET` の値はマスクされて表示される

### Requirement: log パスの指定

`--log <path>` が指定された場合、`full.log` の保存先はそのパスでなければならない（MUST）。未指定の場合はジョブディレクトリ配下の `full.log` としなければならない（MUST）。

#### Scenario: log パスの上書き
Given `agent-exec run --log /tmp/agent.log -- <cmd>` を実行する
When ログが書き込まれる
Then `/tmp/agent.log` に `full.log` が保存される

### Requirement: progress-every の扱い

`--progress-every` が指定された場合、監視プロセスはその間隔以内に `state.json.updated_at` を更新しなければならない（MUST）。stdout に追加の JSON を出力してはならない（MUST）。

#### Scenario: progress 更新
Given `agent-exec run --progress-every 5 -- <cmd>` を実行する
When 5 秒経過する
Then `state.json.updated_at` が更新されている

### Requirement: run/start/tail の JSON range 契約

`run` / `start` / `tail` の JSON には `stdout_log_path` と `stderr_log_path` を含めなければならない（MUST）。
また `stdout_range` / `stderr_range` と `stdout_total_bytes` / `stderr_total_bytes` を含め、`[begin, end)` の half-open interval 契約を満たさなければならない（MUST）。

#### Scenario: tail のログパスと range メトリクス

Given `agent-exec tail <job_id> --max-bytes 128` を実行する
When ログ末尾が取得される
Then `stdout_log_path` と `stderr_log_path` が含まれる
And `stdout_range` と `stderr_range` が含まれる
And `stdout_total_bytes` と `stderr_total_bytes` が 0 以上の整数で返る

### Requirement: 人間向け runtime 制御時間は秒単位である

`run`、`create`、および `_supervise` の人間向け runtime 制御時間オプション (`--timeout`, `--kill-after`, `--progress-every`) は秒単位で解釈しなければならない（MUST）。内部実装でミリ秒へ変換してもよいが、help、README、skills、統合テストは秒単位の契約で一致しなければならない（MUST）。

#### Scenario: run timeout is interpreted in seconds

Given `agent-exec run --timeout 30 -- sh -c "sleep 60"` を実行する
When runtime timeout が適用される
Then `30` は 30 秒として解釈される
And 30 ミリ秒として扱われない

#### Scenario: create persists second-based runtime controls

Given `agent-exec create --timeout 30 --kill-after 5 --progress-every 1 -- sh -c "sleep 60"` を実行する
When job definition が保存される
Then これらの人間向け runtime 制御値は秒単位契約として保存される

### Requirement: 旧 snapshot-era field は正規 surface に残さない

`snapshot` / `final_snapshot` / `stdout_tail` / `stderr_tail` / `*_observed_bytes` / `*_included_bytes` は現行 CLI の正規 help、README、skills、統合テストに残してはならない（MUST NOT）。
現行 `run` は inline output を返し、`tail` は同一 field 名で tail 範囲を返さなければならない（MUST）。

#### Scenario: removed snapshot option is rejected

Given `agent-exec run --snapshot-after 10 -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する

#### Scenario: skills no longer teach snapshot-era fields

Given `skills/agent-exec/**` を参照する
When 現行 run/tail 例を確認する
Then live 例は `snapshot` や `stdout_tail` を使わない

### Requirement: Unix shell-wrapper exec handoff for argv-mode launches

When `run` executes commands through a shell wrapper, the effective wrapper must still be resolved from CLI overrides, config files, or built-in defaults (MUST). On Unix-like platforms, single-string command mode may continue to run as a shell command string, but argv-style invocations with more than one argument must use the resolved shell wrapper only as a launch handoff and must replace the wrapper process with the target argv workload via `exec` semantics (MUST).

#### Scenario: argv-style run uses shell-wrapper exec handoff on Unix

Given a Unix-like platform with the default shell wrapper
When `agent-exec run -- cflx run` is executed
Then the job still launches through the resolved shell wrapper
And the wrapper replaces itself with the target argv workload for completion tracking

#### Scenario: single-string run preserves shell-string semantics

Given a Unix-like platform with the default shell wrapper
When `agent-exec run -- 'echo hello && echo world'` is executed
Then the job runs as a shell command string through the resolved wrapper
And shell syntax remains available to that command string


### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30 秒までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <seconds>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

待機期限指定は秒単位の `--until` に統一しなければならない（MUST）。`--timeout-ms` は有効なオプションとして受け付けてはならない（MUST NOT）。

`wait` のポーリング間隔は秒単位の `--poll <seconds>` で指定できなければならない（MUST）。この間隔は観測用の近似値であり、ミリ秒精度の厳密なチェック時刻を保証してはならない（MUST NOT）。

#### Scenario: wait uses the default 30 second deadline

Given a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
When `agent-exec wait <job_id>` is executed
Then the wait returns within approximately 30 seconds
And if the job finished within the deadline, the response state is terminal

#### Scenario: wait --until returns while the job keeps running

Given a running job created by `agent-exec run -- sh -c "sleep 10"`
When `agent-exec wait --until 1 <job_id>` is executed
Then the response state is `created` or `running`
And `exit_code` is absent

#### Scenario: wait --forever preserves unbounded waiting

Given a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
When `agent-exec wait --forever <job_id>` is executed
Then the response state is terminal after the job exits

#### Scenario: wait --until and --forever are mutually exclusive

Given a user executes `agent-exec wait --until 1 --forever <job_id>`
When clap validates arguments
Then the command fails with usage error

#### Scenario: wait exposes second-based poll option

Given a user inspects `agent-exec wait --help`
When the polling option is shown
Then the canonical polling option is documented in seconds
And the help text does not imply millisecond-accurate checking

#### Scenario: wait rejects removed timeout-ms spelling

Given a user executes `agent-exec wait --timeout-ms 100 <job_id>`
When clap validates arguments
Then the command fails with usage error
And stdout is empty


### Requirement: 環境変数の注入

デフォルトは `inherit-env` を有効としなければならない（MUST）。`--inherit-env` と `--no-inherit-env` は同時指定不可としなければならない（MUST）。`--env-file` は指定順で適用し、`--env` はその後に上書きされなければならない（MUST）。

`run` と `create` が受け付ける definition-time option は、同じ persisted job definition を表す限り同じ metadata 意味論に落ちなければならない（MUST）。これには stdin 定義も含まれる（MUST）。`--stdin <VALUE>` と `--stdin-file <PATH>` は `run` と `create` の両方で受け付けられ、後続 `start` が追加指定なしで同じ入力を再利用できるよう persisted definition に保存されなければならない（MUST）。

`--stdin -` は呼び出し元の非対話 stdin を EOF まで読み切って materialize しなければならない（MUST）。`--stdin <STRING>` はその文字列を UTF-8 バイト列として materialize しなければならない（MUST）。`--stdin-file <PATH>` は指定ファイル内容を実行前に job directory へコピーして materialize しなければならない（MUST）。`start` は persisted stdin 定義を使って child stdin を構築し、未指定時は従来どおり null stdin を維持しなければならない（MUST）。

`--stdin -` が指定されたのに呼び出し元 stdin が tty の場合、`run` / `create` はハングせず stable API error `stdin_required` で失敗しなければならない（MUST）。`--stdin` と `--stdin-file` は同時指定を許可してはならない（MUST NOT）。

#### Scenario: run がヒアドキュメントを child stdin に渡す

Given `agent-exec run --stdin - -- cat <<'EOF'` で複数行のヒアドキュメントが渡される
When ジョブが終了する
Then 終了時の stdout ログ末尾にヒアドキュメント内容が含まれる

#### Scenario: create した stdin を start が再利用する

Given `agent-exec create --stdin "hello" -- cat` を実行する
When 後続で `agent-exec start <job_id> --wait` を実行する
Then 終了時の stdout ログ末尾に `hello` が含まれる
And `start` は追加の stdin 指定を要求しない

#### Scenario: stdin-file は materialized コピーを使う

Given `agent-exec run --stdin-file ./input.txt -- cat` を実行する
When ジョブが起動される
Then child stdin は job directory 内へコピーされた入力内容を使う
And 元の `./input.txt` パスへ実行時依存しない

#### Scenario: tty の --stdin - は即失敗する

Given 呼び出し元 stdin が tty である
When `agent-exec run --stdin - -- cat` を実行する
Then ジョブは起動前に失敗する
And `error.code` は `stdin_required` である

#### Scenario: stdin definition option は create と run で排他規則が一致する

Given `--stdin value --stdin-file ./input.txt` が指定される
When `agent-exec run` または `agent-exec create` の CLI 引数を検証する
Then どちらも usage error で失敗する

## Requirements

### Requirement: 人間向け待機期限オプションは秒単位である

`wait` が受け付ける人間向け待機期限オプションは秒単位で解釈しなければならない（MUST）。既定の待機期限は 30 秒でなければならない（MUST）。内部実装でミリ秒や `Duration` に変換してもよいが、CLI 契約・ヘルプ・ドキュメント・統合テストは秒単位を正規表現として扱わなければならない（MUST）。

#### Scenario: wait uses second-based until

**Given**: a running job created by `agent-exec run -- sh -c "sleep 10"`
**When**: `agent-exec wait --until 30 <job_id>` is executed
**Then**: the command interprets `30` as 30 seconds
**And**: the wait deadline is not interpreted as 30 milliseconds

### Requirement: 人間向けポーリング間隔オプションは秒単位である

`wait` の人間向けポーリング間隔オプションは秒単位で表現しなければならない（MUST）。ポーリングは観測用の近似間隔であり、ミリ秒精度の厳密なチェック時刻を保証してはならない（MUST NOT）。

#### Scenario: wait exposes second-based poll option

**Given**: a user inspects `agent-exec wait --help`
**When**: the polling option is shown
**Then**: the canonical polling option is documented in seconds
**And**: the help text does not imply millisecond-accurate checking


### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30 秒までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <seconds>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

待機期限指定は秒単位の `--until` に統一しなければならない（MUST）。`--timeout-ms` は有効なオプションとして受け付けてはならない（MUST NOT）。

#### Scenario: wait uses the default 30 second deadline

**Given**: a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
**When**: `agent-exec wait <job_id>` is executed
**Then**: the wait returns within approximately 30 seconds
**And**: if the job finished within the deadline, the response state is terminal

#### Scenario: wait --until returns while the job keeps running

**Given**: a running job created by `agent-exec run -- sh -c "sleep 10"`
**When**: `agent-exec wait --until 1 <job_id>` is executed
**Then**: the response state is `created` or `running`
**And**: `exit_code` is absent

#### Scenario: wait --forever preserves unbounded waiting

**Given**: a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
**When**: `agent-exec wait --forever <job_id>` is executed
**Then**: the response state is terminal after the job exits

### Requirement: 人間向け runtime 制御時間は秒単位である

`run`、`create`、および同じ人間向け CLI surface を共有する関連サブコマンドが受け付ける runtime 制御時間オプション (`--timeout`, `--kill-after`, `--progress-every`) は秒単位で解釈しなければならない（MUST）。内部実装でミリ秒へ変換してもよいが、clap help、README、skills、統合テストは秒単位を正規表現として扱わなければならない（MUST）。

#### Scenario: run timeout is interpreted in seconds

**Given**: a user executes `agent-exec run --timeout 30 -- sh -c "sleep 60"`
**When**: the runtime limit is applied
**Then**: `30` is interpreted as 30 seconds
**And**: it is not interpreted as 30 milliseconds

#### Scenario: create persists second-based runtime controls

**Given**: a user executes `agent-exec create --timeout 30 --kill-after 5 --progress-every 1 -- sh -c "sleep 60"`
**When**: the persisted job definition is created
**Then**: the human-facing contract for those values is seconds

### Requirement: 削除済み snapshot-era field は正規 surface に残さない

削除済みの `snapshot-after` フラグは受け付けてはならない（MUST NOT）。snapshot-era field 名（`snapshot` / `final_snapshot` / `stdout_tail` / `stderr_tail` / `*_observed_bytes` / `*_included_bytes`）は現行 CLI の正規 help、README、skills、統合テストに残してはならない（MUST NOT）。現行の `run` は既定で inline output を返し、`tail` は同一 field 名で末尾観測を返さなければならない（MUST）。

#### Scenario: removed snapshot-after option is rejected

**Given**: a user executes `agent-exec run --snapshot-after 10 -- echo hi`
**When**: CLI arguments are validated
**Then**: the command fails with usage error

#### Scenario: skills no longer teach snapshot-era fields

**Given**: a user reads `skills/agent-exec/**`
**When**: they look for current run examples
**Then**: the live examples do not use `snapshot` or `stdout_tail`


### Requirement: tail が range 付き末尾観測を担う

`tail` はログ末尾の観測を担い、`stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` を返さなければならない（MUST）。`run` / `start` の head 契約と field 名は共有するが、返却する byte 区間は末尾側でなければならない（MUST）。

#### Scenario: tail が末尾 API である

Given 実行中または完了済みのジョブが存在する
When `agent-exec tail <job_id> --tail-lines 10 --max-bytes 1024` を実行する
Then `stdout` / `stderr` と range 情報が返る
And `stdout_range[1]` は `stdout_total_bytes` 以下である
And range から返却内容が末尾側であることを判定できる

### Requirement: run/status/tail/wait/kill の JSON

`run`, `start`, `tail` の JSON には `stdout_log_path` と `stderr_log_path` を含めなければならない（MUST）。`run` / `start` / `tail` が本文を返す場合、canonical field は `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding` でなければならない（MUST）。削除済み snapshot-era field 名を新契約として返してはならない（MUST NOT）。

#### Scenario: run は inline output とログパスを返す

Given `agent-exec run -- git diff --staged` を実行する
When `run` の JSON が返る
Then `stdout` と `stdout_range` と `stdout_total_bytes` が含まれる
And `stdout_log_path` と `stderr_log_path` が含まれる
And `snapshot` と `final_snapshot` は含まれない


### Requirement: run のジョブ生成と初回 inline output

`run` はジョブを起動し、既定では `--wait --until 10` 相当の待機予算内で観測できた stdout / stderr を初回レスポンスに含めなければならない（MUST）。`--wait` は人間向け CLI では裸指定だけで `true` として受理されなければならない（MUST）。後方互換のため `--wait true|false` も受け付けてよい（MAY）。`--no-wait` は `--wait false --until 0` のエイリアスであり、追加待機なしの launch-only 返却を明示的に選べなければならない（MUST）。

#### Scenario: run accepts bare wait flag

**Given**: a user executes `agent-exec run --wait -- echo hi`
**When**: CLI arguments are validated and the command runs
**Then**: the command succeeds instead of failing with a missing boolean value error
**And**: the effective wait behavior matches `agent-exec run --wait true -- echo hi`

#### Scenario: run preserves explicit boolean compatibility

**Given**: a user executes `agent-exec run --wait false -- echo hi`
**When**: CLI arguments are validated and the command runs
**Then**: the command succeeds
**And**: the effective wait behavior remains equivalent to `--no-wait`

### Requirement: run/start の観測責務

`run` と `start` は launch-only ではなく、既定では `--wait --until 10` 相当の待機予算内で初回レスポンスに inline output を含めなければならない（MUST）。`run` / `start` の人間向け CLI surface では `--wait` を裸指定だけで `true` として受理しなければならない（MUST）。`--wait true|false` は後方互換として受理してよい（MAY）。`--no-wait` は `--wait false --until 0` のエイリアスとして受け付けなければならない（MUST）。

#### Scenario: start accepts bare wait flag

**Given**: a job created by `agent-exec create -- sh -c "printf 'abc'"` exists
**When**: `agent-exec start --wait <job_id>` is executed
**Then**: the command succeeds instead of failing with a missing boolean value error
**And**: the effective wait behavior matches `agent-exec start --wait true <job_id>`

#### Scenario: start preserves explicit false compatibility

**Given**: a job created by `agent-exec create -- sh -c "sleep 60"` exists
**When**: `agent-exec start --wait false <job_id>` is executed
**Then**: the command succeeds
**And**: the effective wait behavior remains equivalent to `agent-exec start --no-wait <job_id>`
