# agent-exec-tests Specification

## Purpose
TBD - created by archiving change define-agent-exec-tests-ci-v0-1. Update Purpose after archive.
## Requirements
### Requirement: CLI 統合テスト

`run`/`status`/`tail`/`wait`/`kill`/`list` の各コマンドについて、stdout が JSON のみであることと必須フィールドの存在を検証する統合テストを用意しなければならない（MUST）。

#### Scenario: list の JSON 検証
Given `agent-exec list` の統合テストを実行する
When コマンドが完了する
Then stdout の JSON に `schema_version`, `ok`, `type`, `jobs` が含まれる

### Requirement: Windows CI

CI は `windows-latest` を含むマトリクスで実行しなければならない（MUST）。

#### Scenario: CI マトリクス
Given CI ワークフロー設定を確認する
When OS マトリクスを読む
Then `windows-latest` が含まれている


### Requirement: CLI 統合テスト

`run`/`status`/`tail`/`wait`/`kill`/`list` の各コマンドについて、stdout が JSON のみであることと必須フィールドの存在を検証する統合テストを用意しなければならない（MUST）。canonical spec で廃止または拒否された CLI オプションについては、その拒否挙動も統合テストで検証しなければならない（MUST）。

#### Scenario: list の JSON 検証
Given `agent-exec list` の統合テストを実行する
When コマンドが完了する
Then stdout の JSON に `schema_version`, `ok`, `type`, `jobs` が含まれる

#### Scenario: wait rejects removed timeout-ms spelling
Given `agent-exec wait --timeout-ms 100 <job_id>` を統合テストから実行する
When コマンドが完了する
Then 終了コードは 2 である
And stdout は空である
And canonical wait-deadline option is tested via `--until`
