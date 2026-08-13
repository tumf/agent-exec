## MODIFIED Requirements

### Requirement: MCP run uses the canonical managed-job lifecycle

MCP `run` tool は必須の non-empty `command` string array と任意の `cwd`、string-to-string `env`、seconds-based `timeout`、seconds-based bounded `until`、inline UTF-8 `stdin` string、server-local `stdin_file` path、および launch-time completion command sink を受け付けなければならない（MUST）。Completion sink が指定された場合、MCP server は canonical run notification metadata として workload launch 前に永続化しなければならない（MUST）。Invalid sink input は workload launch 前に protocol-safe error として拒否しなければならない（MUST）。`stdin` と `stdin_file` は同時指定を許可してはならず（MUST NOT）、競合時は job を作成する前に protocol-safe error を返さなければならない（MUST）。

MCP `run` の成功結果は CLI `run` と同じ `type="run"` response envelope を含まなければならない（MUST）。Completion sink を永続化した job が non-terminal state で返る場合、結果は optional structured `notification` object を含み、`state="armed"`、generic sink classifications、`polling_required=false`、および completion 時にconfigured sinkへ通知されるため repeated `wait`/`status`/`tail` polling が不要であることを示す message を返さなければならない（MUST）。Notification objectはsession、chat、originating clientなど特定client/hostの概念を含んではならない（MUST NOT）。Sink が永続化されていない場合、または admission が失敗した場合、結果は notification が armed であると示してはならない（MUST NOT）。`armed` は terminal dispatch 用sinkが永続化されたことを意味し、downstream delivery成功を保証してはならない（MUST NOT）。

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
