## MODIFIED Requirements

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `abandon_job_after` and explicit `acknowledge_result_loss`、seconds-based bounded `until`、inline UTF-8 `stdin` string、server-local `stdin_file` path、および launch-time completion command sink を受け付けなければならない（MUST）。Completion sink が指定された場合、MCP server は canonical run notification metadata として workload launch 前に永続化しなければならない（MUST）。Invalid sink input は workload launch 前に protocol-safe error として拒否しなければならない（MUST）。`stdin` と `stdin_file` は同時指定を許可してはならず（MUST NOT）、競合時は job を作成する前に protocol-safe error を返さなければならない（MUST）。MCP stdio transport は protocol 専用であり、managed command の stdin として読み取ってはならない（MUST NOT）。そのため MCP の `stdin` string は値が `"-"` でも caller-stdin marker と解釈せず、literal UTF-8 bytes として扱わなければならない（MUST）。

MCP `run` の inline および file-backed stdin は CLI `run` と同じ bounded job-local materialization、`meta.json.stdin_file` persistence、detached supervisor handoff を使わなければならない（MUST）。`stdin_file` は MCP server process から読める path として扱い、child launch 前に job directory へ snapshot しなければならない（MUST）。両フィールドが省略された場合は managed child の stdin を null のまま維持し、MCP transport から暗黙 capture してはならない（MUST NOT）。入力が既存の stdin byte limit を超える場合、または file が読み取れない場合、child launch 前に canonical error result で失敗しなければならない（MUST）。

実効 `until` は明示 tool value、`AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS`、既存の 10 seconds default の順で最初に利用可能な値を選択し、その後 `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` が設定されている場合は `min(selected, maximum)` に丸めなければならない（MUST）。最大値を超える有効な明示値を error として拒否してはならない（MUST NOT）。有効な call は CLI `run` と同じ persisted job definition、detached supervisor launch、inline observation 契約を使わなければならない（MUST）。MCP surface は command/cwd/env/abandon_job_after/acknowledge_result_loss/until/stdin/stdin_file/completion sink 以外の definition-time controls を受け付けてはならない（MUST NOT）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含み、`job_id`, `state`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `stdout_log_path`, `stderr_log_path` を返さなければならない（MUST）。Completion sink を永続化した job が non-terminal state で返る場合、結果は optional structured `notification` object を含み、`state="armed"`、generic sink classifications、`polling_required=false`、および completion 時に configured sink へ通知されるため repeated `wait`/`status`/`tail` polling が不要であることを示す message を返さなければならない（MUST）。Notification object は session、chat、originating client など特定 client/host の概念を含んではならない（MUST NOT）。Sink が永続化されていない場合、または admission が失敗した場合、結果は notification が armed であると示してはならない（MUST NOT）。`armed` は terminal dispatch 用 sink が永続化されたことを意味し、downstream delivery 成功を保証してはならない（MUST NOT）。Client adapter が宛先付き message delivery を提供する場合、宛先は各 run の persisted command sink に明示しなければならず（MUST）、managed-child `env`、server-global cache、または以前の request から推測してはならない（MUST NOT）。

`abandon_job_after` の description は、この制御がjobを諦めて未完了成果を永久に失う可能性があり、待機だけを止める場合は `until` を使う旨の警告から始めなければならない（MUST）。non-null `abandon_job_after` は `acknowledge_result_loss=true` を要求し、欠落またはfalseならjob作成前に拒否しなければならない（MUST）。legacy `timeout` / `timeout_ms` はjob作成前に拒否し、protocol-safe errorは `abandon_job_after` と `until` の両方を示さなければならない（MUST）。`until` expiryはmanaged jobへsignalしてはならない（MUST NOT）。

#### Scenario: configured run default is used when until is omitted

**Given**: the MCP server has `AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS=20` and no maximum
**When**: the client calls `run` without `until`
**Then**: inline observation is bounded to 20 seconds

#### Scenario: over-maximum run is rounded down

**Given**: the MCP server has `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`
**When**: the client calls `run` with `until=100`
**Then**: the tool proceeds using an effective `until` of 55 seconds
**And**: it returns a successful canonical run envelope instead of an over-maximum error
**And**: the managed job remains detached if the effective observation deadline expires

#### Scenario: maximum caps the legacy run default

**Given**: no default environment variable and `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=5`
**When**: the client calls `run` without `until`
**Then**: the legacy 10-second default is rounded down to 5 seconds

#### Scenario: MCP run rejects an empty command without creating a job

**Given**: an MCP client is connected to an isolated jobs root
**When**: it calls `run` with an empty command array
**Then**: the call returns a protocol-safe error result
**And**: no new job directory is created

#### Scenario: MCP run passes inline stdin through the canonical lifecycle

**Given**: an MCP client calls `run` for a command that echoes stdin with `stdin="alpha\nbeta\n"`
**When**: the managed job finishes
**Then**: child stdout contains the exact supplied UTF-8 bytes
**And**: the job directory contains the same bytes in canonical stdin materialization
**And**: `meta.json.stdin_file` identifies that job-local input

#### Scenario: MCP run snapshots a server-local stdin file

**Given**: a readable server-local file contains known bytes
**When**: an MCP client calls `run` with `stdin_file` set to that path
**Then**: the file is copied into the job directory before child launch
**And**: the child receives the copied bytes
**And**: later modification of the source file does not change the job-local input

#### Scenario: MCP run rejects conflicting stdin definitions

**Given**: an MCP client is connected to an isolated jobs root
**When**: it calls `run` with both `stdin` and `stdin_file`
**Then**: the call returns a protocol-safe error result
**And**: no new job directory is created

#### Scenario: MCP run does not consume protocol transport as job stdin

**Given**: an MCP client calls `run` without `stdin` or `stdin_file`
**When**: the managed command reads stdin
**Then**: the child observes EOF from null stdin
**And**: subsequent MCP JSON-RPC messages remain available to the MCP server

#### Scenario: dash is literal MCP inline input

**Given**: an MCP client calls `run` with `stdin="-"`
**When**: the managed command reads stdin
**Then**: the child receives one literal dash byte
**And**: the MCP server does not wait for a second caller-stdin stream

#### Scenario: invalid MCP stdin fails before child launch

**Given**: MCP `run` receives oversized inline input or a missing, unreadable, or oversized `stdin_file`
**When**: canonical stdin materialization is attempted
**Then**: the call returns an error result before launching the child
**And**: no managed workload process is started

#### Scenario: MCP run reports persisted completion notification as armed

**Given**: an MCP client supplies a valid launch-time completion command sink
**When**: `run` admits the job, persists notification metadata, launches the workload, and returns while it is still running
**Then**: the response includes `notification.state="armed"`
**And**: `notification.polling_required` is `false`
**And**: the response tells the agent that completion will notify it and repeated `wait`, `status`, or `tail` polling is unnecessary
**And**: `meta.json` already contains the same completion sink before the response is returned

#### Scenario: MCP run without a sink does not claim notification is armed

**Given**: an MCP client calls `run` without a completion sink
**When**: the job remains running beyond inline observation
**Then**: the response does not contain `notification.state="armed"`
**And**: the client may choose its own later observation or notification strategy

#### Scenario: invalid launch-time sink fails before workload launch

**Given**: an MCP client supplies invalid completion notification input
**When**: MCP `run` validates admission
**Then**: the call returns a protocol-safe error
**And**: no workload process is launched
**And**: no response claims notification is armed

#### Scenario: explicit client delivery target remains request-scoped

**Given**: an MCP client configures a command sink adapter that requires a message destination
**When**: it calls `run` for a detached workload
**Then**: the destination is embedded explicitly in that job's persisted command sink
**And**: managed-child environment values are not treated as completion-sink environment
**And**: an absent destination makes the adapter fail closed rather than reusing or guessing a route

#### Scenario: completion adapter failure does not alter terminal state

**Given**: an admitted MCP job has a persisted completion command sink
**And**: the downstream client adapter exits non-zero
**When**: the workload reaches a successful terminal state
**Then**: the workload remains successfully terminal
**And**: canonical delivery results record the adapter failure separately

#### Scenario: MCP rejects unacknowledged abandonment

**Given**: the client calls `run` with `abandon_job_after=1` without true acknowledgement
**When**: request input is validated
**Then**: the tool returns a protocol-safe result-loss error
**And**: no job is created

#### Scenario: MCP legacy timeout error is actionable

**Given**: the client calls `run` with `timeout=1`
**When**: request input is validated
**Then**: the protocol-safe error names `abandon_job_after` and `until`
**And**: no job is created

#### Scenario: MCP abandonment and observation remain distinct

**Given**: one request has acknowledged `abandon_job_after=1` and another has `until=1`
**When**: each deadline expires
**Then**: the abandonment request terminates its workload
**And**: the observation request returns a running detached job without signaling it
