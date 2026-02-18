# agent-exec-tests Specification

## Purpose
TBD - created by archiving change define-agent-exec-tests-ci-v0-1. Update Purpose after archive.
## Requirements
### Requirement: CLI 統合テスト

`run`/`status`/`tail`/`wait`/`kill` の各コマンドについて、stdout が JSON のみであることと必須フィールドの存在を検証する統合テストを用意しなければならない（MUST）。

#### Scenario: run の JSON 検証
Given `agent-exec run -- <cmd>` の統合テストを実行する
When コマンドが完了する
Then stdout の JSON に `schema_version`, `ok`, `type` が含まれる

### Requirement: Windows CI

CI は `windows-latest` を含むマトリクスで実行しなければならない（MUST）。

#### Scenario: CI マトリクス
Given CI ワークフロー設定を確認する
When OS マトリクスを読む
Then `windows-latest` が含まれている

