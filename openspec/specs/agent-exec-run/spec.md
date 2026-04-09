# agent-exec-run Specification

## Purpose
TBD - created by archiving change define-agent-exec-run-supervise-v0-1. Update Purpose after archive.
## Requirements
### Requirement: run の監視分離

Issue `#5` verification must distinguish between visible success output and actual workload termination. A job must not be considered reliably complete merely because its logs contain apparent success lines, and regressions for lingering `running` state must include a reproduction shape where the wrapped workload process itself may remain alive after success-like output (MUST).

#### Scenario: cflx-like workload logs success before job leaves running

Given a workload launched via `agent-exec run --snapshot-after 0 -- <workload>` emits success-like completion lines to stdout
And the job still has a live wrapped workload process after those lines are visible
When `agent-exec status <job_id>` and `agent-exec wait <job_id>` are evaluated for issue `#5`
Then the regression analysis must treat this as a distinct failure shape from descendant-held stdio only
And any accepted fix must be verified against this workload-liveness case, not only shell-only synthetic cases

### Requirement: snapshot/tail の末尾取得

`run` の `snapshot` と `tail` は `stdout.log`/`stderr.log` の末尾から生成しなければならない（MUST）。`tail-lines` と `max-bytes` の両制約で切り詰め、`encoding="utf-8-lossy"` を返さなければならない（MUST）。

#### Scenario: tail の制約適用
Given `agent-exec tail <job_id> --lines 10 --max-bytes 1024` を実行する
When ログ末尾が取得される
Then `stdout_tail`/`stderr_tail` は制約内の内容であり `encoding` が含まれる

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

`run` が受け付ける definition-time option は、同じ persisted job definition を表す限り `create` でも受け付けなければならない（MUST）。そのような option は `run` と `create` の両方で同じ `meta.json` 意味論に落ちるよう定義しなければならない（MUST）。一方で `snapshot-after`, tail 制約, `--wait` のような観測用 option は `run` 固有の launch/observation-time option として扱ってよい（MAY）。

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
Given `agent-exec run --progress-every 5s -- <cmd>` を実行する
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

### Requirement: run の既定スナップショットと出力含有

`run` は既定で `snapshot` を返さなければならない（MUST）。既定の待機時間は `snapshot-after=10000ms` 相当とし、`snapshot` の `stdout_tail`/`stderr_tail` は `tail-lines` と `max-bytes` の制約に従って末尾を含めなければならない（MUST）。`snapshot-after=0` のときは従来どおり `snapshot` を省略してよい（MAY）。

#### Scenario: 既定 run は最大 10 秒待機する

Given `agent-exec run -- ping localhost` を実行する
When `run` の JSON が返る
Then `snapshot` が存在する
And `waited_ms` は 10,000 以下である

### Requirement: 改行なし出力の捕捉

`stdout.log` と `stderr.log` は各ストリームの出力バイト列をそのまま追記保存しなければならない（MUST）。`run` の `snapshot` は改行の有無に関わらず `stdout`/`stderr` の末尾を含めなければならない（MUST）。`full.log` の行形式（`<RFC3339> [STDOUT|STDERR] <line>`）は維持する（MUST）。

#### Scenario: 改行なし stdout でも snapshot に含まれる

Given `agent-exec run --snapshot-after 200 --max-bytes 64 -- sh -c "printf 'abc'"` を実行する
When `run` の JSON が返る
Then `snapshot.stdout_tail` に `abc` が含まれる

### Requirement: run と tail の bytes メトリクスの一貫性

MUST: `run` の `snapshot` と `tail` は、`stdout_observed_bytes`/`stderr_observed_bytes` と
`stdout_included_bytes`/`stderr_included_bytes` を同一の算出規則に基づいて返さなければならない。
MUST: 算出規則は既存要件に従い、`observed_bytes` は取得時点のログファイルサイズ、
`included_bytes` は JSON に含めた `*_tail` の UTF-8 bytes 長を示す。

#### Scenario: bytes メトリクスの一貫性

Given 同一ジョブに対して `run` の `snapshot` と `tail` を取得する
When 取得時点のログファイルサイズが観測される
Then `run` と `tail` の `*_observed_bytes` と `*_included_bytes` は同一の規則で算出される

### Requirement: run の同期待機オプション

`run` は `--wait` が指定された場合、既定では最大 30,000ms までジョブの状態変化を待機しなければならない（MUST）。待機上限は `--until <ms>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態 (`exited|killed|failed`) になるまで無制限に待機しなければならない（MUST）。

`--until` と `--forever` は `--wait` と組み合わせる観測用オプションであり、同時指定してはならない（MUST）。`--wait` なしで `--until` / `--forever` を受け付けてはならない（MUST）。

待機期限到達時の `run` JSON はジョブを継続実行したまま、その時点の非終端 `state` (`created|running`) を返さなければならない（MUST）。この場合 `final_snapshot` / `finished_at` / `exit_code` は含めてはならない（MUST）。終端到達時のみ `final_snapshot` と `finished_at`（および存在する場合の `exit_code`）を返さなければならない（MUST）。

`--wait` 指定時の `waited_ms` は、終端到達または待機期限到達までの待機時間を示さなければならない（MUST）。`--wait` 指定時は `snapshot-after` の待機上限 (10,000ms) を適用してはならない（MUST）。

#### Scenario: --wait 既定期限で未完了ジョブを返す

Given `agent-exec run --wait --snapshot-after 0 -- sleep 60` を実行する
When 約 30 秒後に `run` の JSON が返る
Then `state` は `running` または `created` である
And `final_snapshot` と `finished_at` は含まれない

#### Scenario: --wait --until で待機上限を短縮する

Given `agent-exec run --wait --until 100 --snapshot-after 0 -- sleep 60` を実行する
When `run` の JSON が返る
Then 返却時間は既定 30 秒より短い
And `state` は `running` または `created` である

#### Scenario: --wait --forever で終了まで待機する

Given `agent-exec run --wait --forever -- sh -c "echo hi"` を実行する
When `run` の JSON が返る
Then `state` は `exited` である
And `final_snapshot.stdout_tail` に `hi` が含まれる
And `finished_at` が含まれる

#### Scenario: --until と --forever の排他

Given `agent-exec run --wait --until 100 --forever -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する

#### Scenario: --wait なしの --until / --forever は拒否される

Given `agent-exec run --until 100 -- echo hi` を実行する
When CLI 引数を検証する
Then usage error で失敗する
And `agent-exec run --forever -- echo hi` も usage error で失敗する

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


### Requirement: run の同期待機オプション

`run` は `--wait` が指定された場合、既定では最大 30,000ms までジョブの状態変化を待機しなければならない（MUST）。待機上限は `--until <ms>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態 (`exited|killed|failed`) になるまで無制限に待機しなければならない（MUST）。

`--until` と `--forever` は `--wait` と組み合わせる観測用 option であり、`--timeout` が表すジョブ実行時間の timeout とは別概念として扱わなければならない（MUST）。`--until` と `--forever` は単独使用を許可してはならず（MUST NOT）、互いに同時指定も許可してはならない（MUST NOT）。

`--wait` 指定時、`run` は待機上限に達しただけではジョブを終了させてはならない（MUST NOT）。終端状態まで到達した場合の `run` JSON は `exit_code`（存在する場合）と `finished_at` と `final_snapshot` を含めなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合、`run` JSON は非終端の `state` を返し、`exit_code` / `finished_at` / `final_snapshot` を含めてはならない（MUST NOT）。`waited_ms` は実際に待機した時間を示さなければならない（MUST）。

#### Scenario: --wait uses the default 30 second deadline

Given `agent-exec run --wait -- sh -c "sleep 1; echo hi"` is executed
When the command finishes within the default wait deadline
Then the response state is `exited`
And `final_snapshot.stdout_tail` contains `hi`
And `finished_at` is present

#### Scenario: --wait --until returns while the job keeps running

Given `agent-exec run --wait --until 100 -- sh -c "sleep 2; echo hi"` is executed
When the wait deadline is reached before the job exits
Then the response state is `created` or `running`
And `finished_at` is absent
And `final_snapshot` is absent
And the job continues running after the `run` command returns

#### Scenario: --wait --forever preserves unbounded waiting

Given `agent-exec run --wait --forever -- sh -c "sleep 1; echo hi"` is executed
When the job eventually exits
Then the response state is `exited`
And `final_snapshot.stdout_tail` contains `hi`

#### Scenario: wait-deadline flags require --wait

Given a user executes `agent-exec run --until 100 -- sh -c "echo hi"`
When clap validates arguments
Then the command fails with usage error

And given a user executes `agent-exec run --forever -- sh -c "echo hi"`
When clap validates arguments
Then the command fails with usage error

#### Scenario: --until and --forever are mutually exclusive

Given a user executes `agent-exec run --wait --until 100 --forever -- sh -c "echo hi"`
When clap validates arguments
Then the command fails with usage error

### Requirement: wait サブコマンドの待機期限オプション

`wait` サブコマンドは既定では最大 30,000ms までジョブの終端状態を待機しなければならない（MUST）。待機上限は `--until <ms>` によって上書きできなければならない（MUST）。`--forever` が指定された場合は終端状態になるまで無制限に待機しなければならない（MUST）。`--until` と `--forever` は互いに同時指定を許可してはならない（MUST NOT）。

待機上限に達してもジョブは終了させてはならない（MUST NOT）。終端状態まで到達した場合は `state` と `exit_code` を返さなければならない（MUST）。待機上限に達してもジョブが非終端状態の場合は非終端の `state` を返し、`exit_code` を含めてはならない（MUST NOT）。

既存の `--timeout-ms` オプションは `--until` に置換する（MUST）。

#### Scenario: wait uses the default 30 second deadline

Given a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
When `agent-exec wait <job_id>` is executed
Then the wait returns within approximately 30 seconds
And if the job finished within the deadline, the response state is terminal

#### Scenario: wait --until returns while the job keeps running

Given a running job created by `agent-exec run -- sh -c "sleep 10"`
When `agent-exec wait --until 100 <job_id>` is executed
Then the response state is `created` or `running`
And `exit_code` is absent

#### Scenario: wait --forever preserves unbounded waiting

Given a running job created by `agent-exec run -- sh -c "sleep 1; echo done"`
When `agent-exec wait --forever <job_id>` is executed
Then the response state is terminal after the job exits

#### Scenario: wait --until and --forever are mutually exclusive

Given a user executes `agent-exec wait --until 100 --forever <job_id>`
When clap validates arguments
Then the command fails with usage error


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
