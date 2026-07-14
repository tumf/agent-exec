### Requirement: stdio MCP managed-job server

`agent-exec` は `mcp` サブコマンドを提供し、stdio transport 上で managed job 操作を MCP server として公開しなければならない（MUST）。`mcp` は任意の `--root <PATH>` を受け付け、未指定時は既存の jobs root 解決規則を使わなければならない（MUST）。MCP protocol message 以外の内容を stdout に書いてはならず（MUST NOT）、診断と logging は stderr に限定しなければならない（MUST）。

#### Scenario: stdio MCP server initializes without stdout corruption

**Given**: `agent-exec mcp --root <isolated_root>` が stdio で起動している
**When**: MCP client が initialize と tools/list を送る
**Then**: stdout は有効な MCP JSON-RPC response だけを返す
**And**: tools/list は `run`, `status`, `tail`, `wait`, `kill` を含む
**And**: protocol 外の diagnostic text は stdout に含まれない

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `timeout`、seconds-based bounded `until` を受け付けなければならない（MUST）。有効な call は CLI `run` と同じ persisted job definition、detached supervisor launch、inline observation 契約を使わなければならない（MUST）。初期 MCP surface は command/cwd/env/timeout/until 以外の definition-time controls を受け付けてはならない（MUST NOT）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含み、`job_id`, `state`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `stdout_log_path`, `stderr_log_path` を返さなければならない（MUST）。

#### Scenario: MCP run starts a persisted job

**Given**: an MCP client is connected to `agent-exec mcp --root <isolated_root>`
**When**: it calls `run` with `command=["echo", "hello"]`
**Then**: the tool result contains an ok `type="run"` envelope with a non-empty `job_id`
**And**: `<isolated_root>/<job_id>/meta.json` and job logs exist
**And**: `agent-exec --root <isolated_root> status <job_id>` can observe the same job

#### Scenario: MCP run rejects an empty command without creating a job

**Given**: an MCP client is connected to an isolated jobs root
**When**: it calls `run` with an empty command array
**Then**: the call returns a protocol-safe error result
**And**: no new job directory is created

### Requirement: MCP observation tools preserve canonical response semantics

MCP は `status(job_id)`, `tail(job_id, lines?, max_bytes?)`, `wait(job_id, until?)` を提供しなければならない（MUST）。各 tool は CLI と同じ job ID resolution と既存 response envelope semantics を使わなければならない（MUST）。`tail` の既定値は 50 lines と 65536 bytes でなければならない（MUST）。

MCP `wait` は既定で最大 30 seconds のみ待機しなければならない（MUST）。`until` は seconds で上書きできなければならない（MUST）。MCP `wait` は無期限待機 mode を公開してはならない（MUST NOT）。期限到達時、job を停止させず（MUST NOT）、non-terminal state を返し exit_code を含めてはならない（MUST NOT）。

#### Scenario: MCP bounded wait leaves the job running

**Given**: MCP `run` started a job that remains running longer than one second
**When**: the client calls `wait(job_id, until=1)`
**Then**: the result is an ok `type="wait"` envelope with a non-terminal state
**And**: `exit_code` is absent
**And**: a subsequent status call confirms the job was not killed by wait

#### Scenario: MCP tail honors caller bounds

**Given**: a managed job has produced more than one line of stdout
**When**: the client calls `tail(job_id, lines=1, max_bytes=128)`
**Then**: the result is an ok `type="tail"` envelope
**And**: it includes canonical stdout/stderr range and total-byte fields
**And**: stdout is bounded by the requested observation limits

### Requirement: MCP transport lifecycle does not control jobs

A managed job started by MCP `run` must remain managed by the detached agent-exec supervisor after the MCP client disconnects (MUST). MCP server shutdown, client disconnect, bounded wait deadline, malformed tool request, and tool error must not implicitly signal a job (MUST NOT).

#### Scenario: client disconnect does not cancel a managed job

**Given**: an MCP client starts a long-running job and receives its job ID
**When**: the client closes the MCP stdio transport without calling `kill`
**Then**: a later CLI or MCP status lookup finds the same job
**And**: the job remains running or later reaches its natural terminal state

### Requirement: MCP kill requires an explicit tool call

MCP `kill(job_id)` must use the canonical kill behavior with TERM and its existing post-signal observation response (MUST). A signal must be sent only when `kill` is explicitly called (MUST).

#### Scenario: explicit MCP kill terminates a running job

**Given**: a running job exists in the MCP server root
**When**: the client calls `kill(job_id)`
**Then**: the result is an ok `type="kill"` envelope
**And**: a later status or wait observes a terminal killed state

#### Scenario: wait deadline does not imply kill

**Given**: a running job exists in the MCP server root
**When**: the client calls bounded `wait` and its deadline elapses
**Then**: no kill response or signal is emitted
**And**: the job remains observable as non-terminal until it exits naturally or an explicit kill occurs

### Requirement: MCP errors preserve stable job-domain error codes

For valid tool shapes that fail during canonical job-domain operations, MCP results must preserve the existing `ok=false` response envelope and stable error code such as `job_not_found`, `ambiguous_job_id`, or `invalid_state` (MUST). Invalid MCP parameter shapes must not invoke job execution or cancellation (MUST NOT).

#### Scenario: MCP status returns job_not_found envelope

**Given**: an MCP client is connected to an isolated jobs root
**When**: it calls `status` for an unknown job ID
**Then**: the tool result contains `ok=false`
**And**: `error.code` is `job_not_found`
