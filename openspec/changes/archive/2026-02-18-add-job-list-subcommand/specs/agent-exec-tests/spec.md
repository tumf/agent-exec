# agent-exec-tests 変更仕様: add-job-list-subcommand

## MODIFIED Requirements

### Requirement: CLI 統合テスト

`run`/`status`/`tail`/`wait`/`kill`/`list` の各コマンドについて、stdout が JSON のみであることと必須フィールドの存在を検証する統合テストを用意しなければならない（MUST）。

#### Scenario: list の JSON 検証
Given `agent-exec list` の統合テストを実行する
When コマンドが完了する
Then stdout の JSON に `schema_version`, `ok`, `type`, `jobs` が含まれる
