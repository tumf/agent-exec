# agent-exec-contract Specification

## MODIFIED Requirements

### Requirement: CLI サブコマンド構成

`agent-exec` は `run`/`status`/`tail`/`wait`/`kill`/`list` に加えて `install-skills` サブコマンドを提供しなければならない（MUST）。

#### Scenario: install-skills の呼び出し形
Given `agent-exec install-skills` を実行する
When コマンドが完了する
Then stdout は `type="install_skills"` を含む JSON を 1 つ出力する
