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

### Requirement: tail がログ末尾観測を担う

ログ末尾の観測は `tail` が担わなければならない（MUST）。`run` と `start` は snapshot を返してはならず（MUST NOT）、`tail-lines` と `max-bytes` による切り詰め契約は `tail` にのみ適用されなければならない（MUST）。

#### Scenario: tail が末尾観測 API である
Given 実行中または完了済みのジョブが存在する
When `agent-exec tail --tail-lines 10 --max-bytes 1024 <job_id>` を実行する
Then `stdout_tail`/`stderr_tail` と `encoding="utf-8-lossy"` が返る
And `run`/`start` のレスポンスには `snapshot` が含まれない

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

`run` が受け付ける definition-time option は、同じ persisted job definition を表す限り `create` でも受け付けなければならない（MUST）。そのような option は `run` と `create` の両方で同じ `meta.json` 意味論に落ちるよう定義しなければならない（MUST）。削除済みの `snapshot-after` や `run --wait` など旧観測オプションを現行 surface として復活させてはならない（MUST NOT）。

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

### Requirement: run/status/tail/wait/kill の JSON

`run` と `tail` の JSON には `stdout_log_path` と `stderr_log_path` を含めなければならない（MUST）。
`run` の `snapshot` および `tail` は、`stdout_observed_bytes`/`stderr_observed_bytes` と
`stdout_included_bytes`/`stderr_included_bytes` を含めなければならない（MUST）。
`observed_bytes` は取得時点のログファイルサイズ（bytes）を示し、
`included_bytes` は JSON に含めた `*_tail` の UTF-8 bytes 長を示す（MUST）。

#### Scenario: tail のログパスと bytes メトリクス

Given `agent-exec tail <job_id> --max-bytes 128` を実行する
When ログ末尾が取得される
Then `stdout_log_path` と `stderr_log_path` が含まれ、
`stdout_observed_bytes` と `stderr_observed_bytes` が 0 以上の整数で返る
And `stdout_included_bytes` と `stderr_included_bytes` が返り、`*_included_bytes` は `*_observed_bytes` を超えない

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

### Requirement: 削除済み snapshot-era guidance は正規 surface に残さない

削除済みの `snapshot-after` およびそれに依存する旧 guidance は、現行 CLI の正規 help、README、skills、統合テストに残してはならない（MUST NOT）。現行 `run` は起動メタデータを即時返却し、観測責務は `wait` / `tail` / `status` に分離しなければならない（MUST）。

#### Scenario: removed snapshot option is rejected

Given `agent-exec run --snapshot-after 10 -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する

#### Scenario: skills no longer teach snapshot-after

Given `skills/agent-exec/**` を参照する
When 現行 run 例を確認する
Then live 例は `--snapshot-after 0` を要求しない

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

待機期限指定は秒単位の `--until` に統一しなければならない（MUST）。ミリ秒前提の旧語彙や旧解釈を残す場合は、互換または拒否の挙動を明示的に定義しなければならない（MUST）。

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

### Requirement: run と start の観測責務削除

`run` と `start` はジョブ起動コマンドとして即時返却しなければならない（MUST）。完了待機は `wait` が担い、出力取得は `tail` が担わなければならない（MUST）。`start --wait` と snapshot 系オプションは受け付けてはならない（MUST NOT）。

#### Scenario: start は snapshot なしで即時返却する

Given `agent-exec create -- sh -c "sleep 1; echo hi"` で作成した job がある
When `agent-exec start <job_id>` を実行する
Then `start` の JSON に `job_id` と初期 state が含まれる
And `snapshot` は含まれない
And `final_snapshot` は含まれない
And 後続の `agent-exec wait <job_id>` と `agent-exec tail <job_id>` で完了待機と出力取得が行える

#### Scenario: start は削除済み観測オプションを拒否する

Given `agent-exec start --snapshot-after 10 <job_id>` を実行する
When CLI 引数を検証する
Then usage error で失敗する

And given `agent-exec start --wait <job_id>` を実行する
When CLI 引数を検証する
Then usage error で失敗する

## Requirements

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

### Requirement: 削除済み snapshot-era guidance は正規 surface に残さない

削除済みの `snapshot-after` およびそれに依存する旧 guidance は、現行 CLI の正規 help、README、skills、統合テストに残してはならない（MUST NOT）。現行の `run` は即時返却し、観測責務は `wait` / `tail` / `status` に分離されていることを正規 docs が示さなければならない（MUST）。

#### Scenario: removed snapshot option is rejected

**Given**: a user executes `agent-exec run --snapshot-after 10 -- echo hi`
**When**: CLI arguments are validated
**Then**: the command fails with usage error

#### Scenario: skills no longer teach snapshot-after

**Given**: a user reads `skills/agent-exec/**`
**When**: they look for current run examples
**Then**: the live examples do not require `--snapshot-after 0` to explain immediate return


### Requirement: run の既定スナップショットと出力含有

`run` は返却前に観測用 snapshot を生成するための追加待機を行ってはならない（MUST NOT）。`run` の主責務は job 起動と `job_id` / 初期 state / ログパスの返却であり、完了待機と出力観測は `wait` / `tail` / `status` に分離しなければならない（MUST）。

#### Scenario: default run returns immediately without snapshot wait

**Given**: `agent-exec run -- sh -c "sleep 1; echo hi"` is executed
**When**: the JSON response is returned
**Then**: `job_id` is present
**And**: `snapshot` is absent
**And**: `final_snapshot` is absent

### Requirement: run は削除済み snapshot オプションを拒否する

`run` は `snapshot-after`、`tail-lines`、`max-bytes`、および削除済み観測系フラグを受け付けてはならない（MUST NOT）。

#### Scenario: run rejects removed snapshot-after option

**Given**: `agent-exec run --snapshot-after 10 -- echo hi` is executed
**When**: CLI arguments are validated
**Then**: the command fails with usage error
