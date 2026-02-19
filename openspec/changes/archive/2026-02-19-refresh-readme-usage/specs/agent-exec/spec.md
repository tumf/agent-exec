# agent-exec Spec Delta (refresh-readme-usage)

## ADDED Requirements
### Requirement: README の利用導線

README は `run/status/tail/wait/kill/list` を対象にしたコピペ可能な使用例を含めなければならない（MUST）。README は stdout が JSON-only であり、stderr が診断ログであることを明記しなければならない（MUST）。

#### Scenario: README のコマンド例

Given リポジトリの `README.md` を読む
When 利用例セクションを確認する
Then `run`/`status`/`tail`/`wait`/`kill`/`list` の例が含まれる
And stdout が JSON-only である旨が明記されている
