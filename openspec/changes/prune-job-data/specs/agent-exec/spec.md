# agent-exec Specification (Change: prune-job-data)

## ADDED Requirements

### Requirement: gc サブコマンド

`agent-exec` は job データを明示的に削除する `gc` サブコマンドを提供しなければならない（MUST）。`gc` は `--older-than <duration>` を任意引数として受け付け、未指定時は既定の `30d` を使用しなければならない（MUST）。`gc` は既定で削除を実行し、`--dry-run` 指定時は削除せず候補のみを報告しなければならない（MUST）。

#### Scenario: 既定の 30d を使う
Given 30 日より古い terminal job が存在する
When `agent-exec gc` を実行する
Then コマンドは `ok=true` の JSON を返す
And 対象 job ディレクトリは root 配下から削除される

#### Scenario: dry-run で候補確認
Given 古い terminal job が存在する
When `agent-exec gc --older-than 7d --dry-run` を実行する
Then コマンドは `ok=true` の JSON を返す
And 対象 job ディレクトリは削除されない

#### Scenario: 既定モードで削除実行
Given 古い terminal job が存在する
When `agent-exec gc --older-than 7d` を実行する
Then コマンドは `ok=true` の JSON を返す
And 対象 job ディレクトリは root 配下から削除される

### Requirement: gc の JSON レスポンス

`gc` は少なくとも `root`, `dry_run`, `older_than`, `older_than_source`, `deleted`, `skipped`, `freed_bytes`, `jobs` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `state`, `action`, `reason`, `bytes` を含まなければならない（MUST）。

#### Scenario: 削除結果の内訳
Given `agent-exec gc --older-than 7d --dry-run` を実行する
When 対象 job とスキップ job が混在している
Then JSON の `jobs` には各 job の `action` と `reason` が含まれる
And `freed_bytes` は削除または削除予定の合計バイト数を表す
