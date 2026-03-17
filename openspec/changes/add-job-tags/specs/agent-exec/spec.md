# agent-exec Specification (Change: add-job-tags)

## MODIFIED Requirements

### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `state`, `started_at`, `tags` を含まなければならない（MUST）。`tags` はタグがない場合でも空配列で返さなければならない（MUST）。`exit_code` と `finished_at` と `updated_at` は存在する場合にのみ含めてよい（MAY）。

#### Scenario: list が tags を含む
Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `state`, `started_at`, `tags` を含む

### Requirement: list のタグフィルタ

`list` は repeatable な `--tag <PATTERN>` を受け付けなければならない（MUST）。各 `PATTERN` は完全一致タグまたは末尾が `.*` の namespace-prefix パターンとして評価しなければならない（MUST）。複数の `--tag` が指定された場合、`list` はすべての tag filter を満たすジョブだけを返さなければならない（MUST）。tag filter は既存の cwd フィルタおよび state フィルタと論理積で合成されなければならない（MUST）。

#### Scenario: 完全一致タグで絞り込む
Given tag `aaa` を持つジョブと持たないジョブが存在する
When `agent-exec list --tag aaa` を実行する
Then `jobs` は tag `aaa` を持つジョブのみを含む

#### Scenario: namespace prefix タグで絞り込む
Given tags `hoge.fuga`, `hoge.fuga.geho`, `hoge.foo` を持つジョブ群が存在する
When `agent-exec list --tag hoge.fuga.*` を実行する
Then `jobs` は `hoge.fuga` または `hoge.fuga.` で始まる tag を持つジョブのみを含む

#### Scenario: 複数 tag filter の AND 条件
Given tag `aaa` のみを持つジョブと tag `aaa` と `bbb` を両方持つジョブが存在する
When `agent-exec list --tag aaa --tag bbb` を実行する
Then `jobs` は `aaa` と `bbb` の両方を満たすジョブのみを含む

## ADDED Requirements

### Requirement: 既存ジョブの tag 設定

`agent-exec` は `tag set <JOB_ID> --tag <TAG>...` を受け付けなければならない（MUST）。`tag set` は既存ジョブの `meta.json.tags` を指定された deduplicate 済み tag 配列で置き換えなければならない（MUST）。`tag set` は metadata-only な操作であり、対象ジョブのプロセス状態や他の永続メタデータを変更してはならない（MUST）。対象ジョブが存在しない場合は既存の JSON error contract で `job_not_found` を返さなければならない（MUST）。

#### Scenario: 実行中ジョブの tags を置き換える
Given 既存ジョブがあり現在の tags が `["old"]` である
When `agent-exec tag set <JOB_ID> --tag aaa --tag bbb` を実行する
Then 成功 JSON の `tags` は `["aaa", "bbb"]` である
And 後続の `list` は更新後の tags でそのジョブを返す

#### Scenario: 存在しない job_id を拒否する
Given `job_id` に対応するジョブが存在しない
When `agent-exec tag set <JOB_ID> --tag aaa` を実行する
Then stdout は `error.code = "job_not_found"` を含む JSON error である
