# agent-exec 変更仕様: add-job-list-subcommand

## ADDED Requirements

### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `state`, `started_at` を含み、`exit_code` と `finished_at` と `updated_at` は存在する場合にのみ含めてよい（MAY）。

#### Scenario: list が必須フィールドを返す
Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `state`, `started_at` を含む

### Requirement: list の並び順と制約

`list` は `started_at` の降順で `jobs` を返さなければならない（MUST）。`--limit` が指定された場合は上限件数まで返し、超過した場合は `truncated=true` を返さなければならない（MUST）。

#### Scenario: limit による切り詰め
Given `agent-exec list --limit 2` を実行する
When ジョブが 3 件以上存在する
Then `jobs` の長さは 2 である
And `truncated` は `true` である

### Requirement: root 不在時の挙動

root が存在しない場合、`list` はエラーではなく `jobs=[]` を返さなければならない（MUST）。

#### Scenario: root が存在しない
Given `agent-exec list --root /path/does/not/exist` を実行する
When コマンドが完了する
Then `jobs` は空配列である
