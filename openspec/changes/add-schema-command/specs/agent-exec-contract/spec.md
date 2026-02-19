# agent-exec-contract Spec Delta (add-schema-command)

## MODIFIED Requirements
### Requirement: CLI サブコマンド構成

`agent-exec` は `schema` サブコマンドを提供しなければならない（MUST）。`schema` は stdout に `type="schema"` の JSON を 1 つ出力しなければならない（MUST）。`schema` の JSON は `schema_format` と `schema` を含み、`schema_format` は `json-schema-draft-07` でなければならない（MUST）。

#### Scenario: schema を取得する

Given `agent-exec schema` を実行する
When コマンドが完了する
Then stdout は `type="schema"` の JSON である
And `schema_format` は `json-schema-draft-07` である
And `schema` は JSON オブジェクトである
