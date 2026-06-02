## MODIFIED Requirements

### Requirement: list の状態フィルタ

`list` は `--state <state>` を受け付け、指定時は `jobs` を `jobs[].state == <state>` に一致するものだけ返さなければならない（MUST）。
`state` の値は `running|exited|killed|failed|unknown` に限定され、未知の値は usage エラーとする（MUST）。
`--state` 指定時はフィルタ適用後の件数に対して `--limit` を適用し、必要に応じて `truncated=true` としなければならない（MUST）。
`list` は persisted status が `running` の job を表示・フィルタする前に、persisted `pid` の生存を best-effort で確認しなければならない（MUST）。persisted `pid` が存在しない、または存在しないプロセスを指している場合、その job の effective state は `unknown` として扱わなければならず、`list --state running` と `ps` に含めてはならない（MUST）。この reconciliation は `state.json` を書き換えてはならない（MUST NOT）。

#### Scenario: 実行中ジョブのみの取得

Given 実行中ジョブと終了済みジョブが存在する
When `agent-exec list --state running` を実行する
Then `jobs` は `state=running` のジョブのみを含む
And `jobs` の全要素で `state` は `running` である

#### Scenario: stale running job is excluded from ps

**Given**: `state.json` の status が `running` で persisted `pid` が存在しない job がある
**When**: `agent-exec ps --all` を実行する
**Then**: その job は `jobs` に含まれない

#### Scenario: stale running job is visible as unknown in list

**Given**: `state.json` の status が `running` で persisted `pid` が存在しない job がある
**When**: `agent-exec list --all` を実行する
**Then**: その job は `jobs` に含まれる
**And**: その job の `state` は `unknown` である

#### Scenario: live running job remains running

**Given**: `state.json` の status が `running` で persisted `pid` が生存している job がある
**When**: `agent-exec ps --all` を実行する
**Then**: その job は `jobs` に含まれる
**And**: その job の `state` は `running` である
