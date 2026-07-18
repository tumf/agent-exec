## MODIFIED Requirements

### Requirement: stdio MCP managed-job server

`agent-exec` は `mcp` サブコマンドを提供し、stdio transport 上で managed job 操作を MCP server として公開しなければならない（MUST）。`mcp` は任意の `--root <PATH>` に加えて、MCP client の request timeout、response safety margin、`run` の既定 `until`、`wait` の既定 `until` を設定する startup options を受け付けなければならない（MUST）。client timeout が設定された場合、server は timeout から safety margin を安全に減算して最大 `until` を導出し、無効または枯渇した budget と最大値を超える既定値を serving 開始前に拒否しなければならない（MUST）。未指定時は既存の jobs root 解決規則と既存の MCP observation defaults を維持しなければならない（MUST）。MCP protocol message 以外の内容を stdout に書いてはならず（MUST NOT）、診断と logging は stderr に限定しなければならない（MUST）。

#### Scenario: stdio MCP server initializes without stdout corruption

**Given**: `agent-exec mcp --root <isolated_root>` が stdio で起動している
**When**: MCP client が initialize と tools/list を送る
**Then**: stdout は有効な MCP JSON-RPC response だけを返す
**And**: tools/list は `run`, `status`, `tail`, `wait`, `kill` を含む
**And**: protocol 外の diagnostic text は stdout に含まれない

#### Scenario: platform timeout configures a safe observation budget

**Given**: an MCP platform has a 60-second request timeout
**When**: `agent-exec mcp` is started with a 60-second client timeout and a 5-second safety margin
**Then**: the maximum permitted `until` is 55 seconds
**And**: the same maximum applies to MCP `run` and MCP `wait`

#### Scenario: invalid startup budget is rejected

**Given**: an MCP startup configuration has a safety margin greater than or equal to its client timeout
**When**: `agent-exec mcp` starts
**Then**: startup fails before serving MCP protocol requests
**And**: stderr identifies the invalid timeout budget

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `timeout`、seconds-based bounded `until` を受け付けなければならない（MUST）。`until` が省略された場合、MCP server startup で設定された `run` default を使い、startup で未設定の場合は 10 seconds を使わなければならない（MUST）。client timeout budget が設定されている場合、明示または既定の `until` が導出された最大値を超える call を job 作成前に拒否しなければならない（MUST）。有効な call は CLI `run` と同じ persisted job definition、detached supervisor launch、inline observation 契約を使わなければならない（MUST）。初期 MCP surface は command/cwd/env/timeout/until 以外の definition-time controls を受け付けてはならない（MUST NOT）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含み、`job_id`, `state`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `stdout_log_path`, `stderr_log_path` を返さなければならない（MUST）。

#### Scenario: MCP run starts a persisted job

**Given**: an MCP client is connected to `agent-exec mcp --root <isolated_root>`
**When**: it calls `run` with `command=["echo", "hello"]`
**Then**: the tool result contains an ok `type="run"` envelope with a non-empty `job_id`
**And**: `<isolated_root>/<job_id>/meta.json` and job logs exist
**And**: `agent-exec --root <isolated_root> status <job_id>` can observe the same job

#### Scenario: configured run default is used when until is omitted

**Given**: the MCP server has a client-safe maximum of 55 seconds and a run default of 55 seconds
**When**: the client calls `run` without `until`
**Then**: inline observation is bounded to 55 seconds
**And**: the managed job remains detached if the observation deadline expires

#### Scenario: over-budget run is rejected before job creation

**Given**: the MCP server has a client-safe maximum of 55 seconds
**When**: the client calls `run` with `until=56`
**Then**: the tool returns an actionable protocol-safe error without waiting 56 seconds
**And**: no job is created

#### Scenario: MCP run rejects an empty command without creating a job

**Given**: an MCP client is connected to an isolated jobs root
**When**: it calls `run` with an empty command array
**Then**: the call returns a protocol-safe error result
**And**: no new job directory is created

### Requirement: MCP observation tools preserve canonical response semantics

MCP は `status(job_id)`, `tail(job_id, lines?, max_bytes?)`, `wait(job_id, until?)` を提供しなければならない（MUST）。各 tool は CLI と同じ job ID resolution と既存 response envelope semantics を使わなければならない（MUST）。`tail` の既定値は 50 lines と 65536 bytes でなければならない（MUST）。

MCP `wait` の `until` が省略された場合、MCP server startup で設定された `wait` default を使い、startup で未設定の場合は最大 30 seconds のみ待機しなければならない（MUST）。`until` は seconds で上書きできなければならない（MUST）。client timeout budget が設定されている場合、明示または既定の `until` が導出された最大値を超える call を observation 開始前に拒否しなければならない（MUST）。MCP `wait` は無期限待機 mode を公開してはならない（MUST NOT）。期限到達時、job を停止させず（MUST NOT）、non-terminal state を返し exit_code を含めてはならない（MUST NOT）。

#### Scenario: MCP bounded wait leaves the job running

**Given**: MCP `run` started a job that remains running longer than one second
**When**: the client calls `wait(job_id, until=1)`
**Then**: the result is an ok `type="wait"` envelope with a non-terminal state
**And**: `exit_code` is absent
**And**: a subsequent status call confirms the job was not killed by wait

#### Scenario: configured wait default is used when until is omitted

**Given**: the MCP server has a client-safe maximum of 55 seconds and a wait default of 55 seconds
**When**: the client calls `wait` without `until`
**Then**: observation is bounded to 55 seconds
**And**: deadline expiry does not signal the managed job

#### Scenario: over-budget wait is rejected without affecting the job

**Given**: a running managed job and an MCP server with a client-safe maximum of 55 seconds
**When**: the client calls `wait` with `until=56`
**Then**: the tool returns an actionable protocol-safe error without waiting 56 seconds
**And**: a subsequent status call observes the same running job

#### Scenario: MCP tail honors caller bounds

**Given**: a managed job has produced more than one line of stdout
**When**: the client calls `tail(job_id, lines=1, max_bytes=128)`
**Then**: the result is an ok `type="tail"` envelope
**And**: it includes canonical stdout/stderr range and total-byte fields
**And**: stdout is bounded by the requested observation limits
