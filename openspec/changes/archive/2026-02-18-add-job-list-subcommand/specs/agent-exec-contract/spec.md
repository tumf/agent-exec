# agent-exec-contract 変更仕様: add-job-list-subcommand

## MODIFIED Requirements

### Requirement: CLI サブコマンド構成

`agent-exec` は `run`/`status`/`tail`/`wait`/`kill`/`list` の 6 サブコマンドを提供しなければならない（MUST）。`list` はジョブ一覧を JSON で返さなければならない（MUST）。

#### Scenario: list の呼び出し形
Given `agent-exec list --root /tmp/jobs` を実行する
When コマンドが完了する
Then stdout は `type="list"` の JSON を 1 つ出力する
