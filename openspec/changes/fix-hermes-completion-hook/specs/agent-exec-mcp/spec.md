## MODIFIED Requirements

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `timeout`、seconds-based bounded `until`、inline UTF-8 `stdin` string、server-local `stdin_file` path、および launch-time completion command sink を受け付けなければならない（MUST）。Completion sink が指定された場合、MCP server は canonical run notification metadata として workload launch 前に永続化しなければならない（MUST）。Invalid sink input は workload launch 前に protocol-safe error として拒否しなければならない（MUST）。`stdin` と `stdin_file` は同時指定を許可してはならず（MUST NOT）、競合時は job を作成する前に protocol-safe error を返さなければならない（MUST）。MCP stdio transport は protocol 専用であり、managed command の stdin として読み取ってはならない（MUST NOT）。そのため MCP の `stdin` string は値が `"-"` でも caller-stdin marker と解釈せず、literal UTF-8 bytes として扱わなければならない（MUST）。

MCP `run` の inline および file-backed stdin は CLI `run` と同じ bounded job-local materialization、`meta.json.stdin_file` persistence、detached supervisor handoff を使わなければならない（MUST）。`stdin_file` は MCP server process から読める path として扱い、child launch 前に job directory へ snapshot しなければならない（MUST）。両フィールドが省略された場合は managed child の stdin を null のまま維持し、MCP transport から暗黙 capture してはならない（MUST NOT）。入力が既存の stdin byte limit を超える場合、または file が読み取れない場合、child launch 前に canonical error result で失敗しなければならない（MUST）。

実効 `until` は明示 tool value、`AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS`、既存の 10 seconds default の順で最初に利用可能な値を選択し、その後 `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` が設定されている場合は `min(selected, maximum)` に丸めなければならない（MUST）。最大値を超える有効な明示値を error として拒否してはならない（MUST NOT）。有効な call は CLI `run` と同じ persisted job definition、detached supervisor launch、inline observation 契約を使わなければならない（MUST）。MCP surface は command/cwd/env/timeout/until/stdin/stdin_file/completion sink 以外の definition-time controls を受け付けてはならない（MUST NOT）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含み、`job_id`, `state`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `stdout_log_path`, `stderr_log_path` を返さなければならない（MUST）。Completion sink を永続化した job が non-terminal state で返る場合、結果は optional structured `notification` object を含み、`state="armed"`、generic sink classifications、`polling_required=false`、および completion 時に configured sink へ通知されるため repeated `wait`/`status`/`tail` polling が不要であることを示す message を返さなければならない（MUST）。Notification object は session、chat、originating client など特定 client/host の概念を含んではならない（MUST NOT）。Sink が永続化されていない場合、または admission が失敗した場合、結果は notification が armed であると示してはならない（MUST NOT）。`armed` は terminal dispatch 用 sink が永続化されたことを意味し、downstream delivery 成功を保証してはならない（MUST NOT）。Client adapter が宛先付き message delivery を提供する場合、宛先は各 run の persisted command sink に明示しなければならず（MUST）、managed-child `env`、server-global cache、または以前の request から推測してはならない（MUST NOT）。

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
