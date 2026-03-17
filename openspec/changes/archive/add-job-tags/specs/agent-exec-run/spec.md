# agent-exec-run Specification (Change: add-job-tags)

## ADDED Requirements

### Requirement: run の repeatable tag 指定

`run` は repeatable な `--tag <TAG>` を受け付けなければならない（MUST）。`TAG` は namespace-like な保存用タグとして検証されなければならない（MUST）。`run` の成功 JSON は deduplicate 済みの `tags` 配列を含まなければならない（MUST）。不正な tag 値は usage エラーとして拒否しなければならない（MUST）。

#### Scenario: run が tags を返す
Given `agent-exec run --tag aaa --tag bbb -- sh -c "echo hi"` を実行する
When `run` の JSON が返る
Then JSON に `tags` が含まれる
And `tags` は `["aaa", "bbb"]` である

#### Scenario: 不正な tag を拒否する
Given `agent-exec run --tag aaa* -- sh -c "echo hi"` を実行する
When コマンドライン引数が検証される
Then コマンドは usage エラーとして終了する
