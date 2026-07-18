## MODIFIED Requirements

### Requirement: stdio MCP managed-job server

`agent-exec` は `mcp` サブコマンドを提供し、stdio transport 上で managed job 操作を MCP server として公開しなければならない（MUST）。`mcp` は任意の `--root <PATH>` を受け付け、未指定時は既存の jobs root 解決規則を使わなければならない（MUST）。MCP server は任意の `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` process environment variable を起動時に読み取り、有効な non-negative integer の場合は MCP `run` と MCP `wait` に共通する既定 `until` および最大 `until` として使わなければならない（MUST）。変数が malformed、negative、fractional、empty、または範囲外の場合は protocol serving 開始前に失敗しなければならない（MUST）。MCP protocol message 以外の内容を stdout に書いてはならず（MUST NOT）、診断と logging は stderr に限定しなければならない（MUST）。

#### Scenario: stdio MCP server initializes without stdout corruption

**Given**: `agent-exec mcp --root <isolated_root>` が stdio で起動している
**When**: MCP client が initialize と tools/list を送る
**Then**: stdout は有効な MCP JSON-RPC response だけを返す
**And**: tools/list は `run`, `status`, `tail`, `wait`, `kill` を含む
**And**: protocol 外の diagnostic text は stdout に含まれない

#### Scenario: host configures one safe observation value

**Given**: an MCP host starts the server with `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`
**When**: the MCP server initializes
**Then**: the shared omitted default and explicit maximum for `run.until` and `wait.until` are 55 seconds
**And**: agent-exec performs no client-timeout or safety-margin calculation

#### Scenario: invalid environment value is rejected

**Given**: `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` is not a valid non-negative integer
**When**: `agent-exec mcp` starts
**Then**: startup fails before serving MCP protocol requests
**And**: stderr identifies the invalid environment variable

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `timeout`、seconds-based bounded `until` を受け付けなければならない（MUST）。`AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` が設定されている場合、`until` 省略時はその値を使い、明示値がその値を超える call は job 作成前に拒否しなければならない（MUST）。環境変数が未設定の場合、`until` 省略時は既存の 10 seconds を使い、明示値に新しい最大制約を課してはならない（MUST NOT）。有効な call は CLI `run` と同じ persisted job definition、detached supervisor launch、inline observation 契約を使わなければならない（MUST）。初期 MCP surface は command/cwd/env/timeout/until 以外の definition-time controls を受け付けてはならない（MUST NOT）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含み、`job_id`, `state`, `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `stdout_log_path`, `stderr_log_path` を返さなければならない（MUST）。

#### Scenario: MCP run starts a persisted job

**Given**: an MCP client is connected to `agent-exec mcp --root <isolated_root>`
**When**: it calls `run` with `command=["echo", "hello"]`
**Then**: the tool result contains an ok `type="run"` envelope with a non-empty `job_id`
**And**: `<isolated_root>/<job_id>/meta.json` and job logs exist
**And**: `agent-exec --root <isolated_root> status <job_id>` can observe the same job

#### Scenario: configured run default is used when until is omitted

**Given**: the MCP server has `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`
**When**: the client calls `run` without `until`
**Then**: inline observation is bounded to 55 seconds
**And**: the managed job remains detached if the observation deadline expires

#### Scenario: over-maximum run is rejected before job creation

**Given**: the MCP server has `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`
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

`AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` が設定されている場合、MCP `wait` は `until` 省略時にその値を使い、明示値がその値を超える call を observation 開始前に拒否しなければならない（MUST）。環境変数が未設定の場合、MCP `wait` は既定で最大 30 seconds のみ待機し、明示値に新しい最大制約を課してはならない（MUST NOT）。`until` は seconds で上書きできなければならない（MUST）。MCP `wait` は無期限待機 mode を公開してはならない（MUST NOT）。期限到達時、job を停止させず（MUST NOT）、non-terminal state を返し exit_code を含めてはならない（MUST NOT）。

#### Scenario: MCP bounded wait leaves the job running

**Given**: MCP `run` started a job that remains running longer than one second
**When**: the client calls `wait(job_id, until=1)`
**Then**: the result is an ok `type="wait"` envelope with a non-terminal state
**And**: `exit_code` is absent
**And**: a subsequent status call confirms the job was not killed by wait

#### Scenario: configured wait default is used when until is omitted

**Given**: the MCP server has `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`
**When**: the client calls `wait` without `until`
**Then**: observation is bounded to 55 seconds
**And**: deadline expiry does not signal the managed job

#### Scenario: over-maximum wait is rejected without affecting the job

**Given**: a running managed job and an MCP server with `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS=55`
**When**: the client calls `wait` with `until=56`
**Then**: the tool returns an actionable protocol-safe error without waiting 56 seconds
**And**: a subsequent status call observes the same running job

#### Scenario: MCP tail honors caller bounds

**Given**: a managed job has produced more than one line of stdout
**When**: the client calls `tail(job_id, lines=1, max_bytes=128)`
**Then**: the result is an ok `type="tail"` envelope
**And**: it includes canonical stdout/stderr range and total-byte fields
**And**: stdout is bounded by the requested observation limits
