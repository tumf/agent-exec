# agent-exec-windows Specification

## Purpose
TBD - created by archiving change define-agent-exec-windows-process-v0-1. Update Purpose after archive.
## Requirements
### Requirement: Job Object によるツリー管理

Windows では `run` が起動した子プロセスを Job Object に割り当て、プロセスツリーを管理しなければならない（MUST）。

#### Scenario: 子プロセスの割り当て
Given Windows 環境で `agent-exec run -- <cmd>` を実行する
When 子プロセスが起動する
Then 子プロセスは Job Object に割り当てられている

### Requirement: kill のシグナルマッピング

`kill` は `TERM|INT|KILL` を受け付け、Windows で未対応のシグナルは `KILL` 相当で処理しなければならない（MUST）。

#### Scenario: TERM の処理
Given Windows 環境で `agent-exec kill <job_id> --signal TERM` を実行する
When コマンドが成功する
Then 対象ジョブのプロセスツリーが終了する

### Requirement: state.json の Windows 情報

Windows では `state.json` に Job Object を識別できる情報を含めなければならない（MUST）。

#### Scenario: state.json の識別子
Given Windows 環境で実行中のジョブがある
When `state.json` を読む
Then Job Object の識別情報が含まれる

