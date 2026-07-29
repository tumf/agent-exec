## MODIFIED Requirements

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `timeout`、seconds-based bounded `until`、inline UTF-8 `stdin` string、server-local `stdin_file` path を受け付けなければならない（MUST）。`stdin` と `stdin_file` は同時指定を許可してはならず（MUST NOT）、競合時は job を作成する前に protocol-safe error を返さなければならない（MUST）。MCP stdio transport は protocol 専用であり、managed command の stdin として読み取ってはならない（MUST NOT）。そのため MCP の `stdin` string は値が `"-"` でも caller-stdin marker と解釈せず、literal UTF-8 bytes として扱わなければならない（MUST）。

MCP `run` の inline および file-backed stdin は CLI `run` と同じ bounded job-local materialization、`meta.json.stdin_file` persistence、detached supervisor handoff を使わなければならない（MUST）。`stdin_file` は MCP server process から読める path として扱い、child launch 前に job directory へ snapshot しなければならない（MUST）。両フィールドが省略された場合は managed child の stdin を null のまま維持し、MCP transport から暗黙 capture してはならない（MUST NOT）。入力が既存の stdin byte limit を超える場合、または file が読み取れない場合、child launch 前に canonical error result で失敗しなければならない（MUST）。

実効 `until` は明示 tool value、`AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS`、既存の 10 seconds default の順で最初に利用可能な値を選択し、その後 `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` が設定されている場合は `min(selected, maximum)` に丸めなければならない（MUST）。最大値を超える有効な明示値を error として拒否してはならない（MUST NOT）。有効な call は CLI `run` と同じ persisted job definition、detached supervisor launch、inline observation 契約を使わなければならない（MUST）。MCP surface は command/cwd/env/timeout/until/stdin/stdin_file 以外の definition-time controls を受け付けてはならない（MUST NOT）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含み、`job_id`, `state`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `stdout_log_path`, `stderr_log_path` を返さなければならない（MUST）。

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
