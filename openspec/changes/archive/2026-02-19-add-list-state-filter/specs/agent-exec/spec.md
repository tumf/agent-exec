# agent-exec Specification (Change: add-list-state-filter)

## ADDED Requirements

### Requirement: list の状態フィルタ

`list` は `--state <state>` を受け付け、指定時は `jobs` を `jobs[].state == <state>` に一致するものだけ返さなければならない（MUST）。
`state` の値は `running|exited|killed|failed|unknown` に限定され、未知の値は usage エラーとする（MUST）。
`--state` 指定時はフィルタ適用後の件数に対して `--limit` を適用し、必要に応じて `truncated=true` としなければならない（MUST）。

#### Scenario: 実行中ジョブのみの取得
Given 実行中ジョブと終了済みジョブが存在する
When `agent-exec list --state running` を実行する
Then `jobs` は `state=running` のジョブのみを含む
And `jobs` の全要素で `state` は `running` である
