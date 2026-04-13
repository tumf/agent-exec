## ADDED Requirements

### Requirement: hash-like job IDs for new jobs

`run`, `create`, および `POST /exec` が新規 job を作成する際の canonical `job_id` は、ULID ではなく固定長の小文字 hex ランダム識別子でなければならない（MUST）。新規 `job_id` は同一 root 配下の既存 job directory 名と衝突してはならず、衝突時は再生成しなければならない（MUST）。

#### Scenario: run creates a hash-like job ID

Given `agent-exec run -- echo hi` を実行する
When ジョブが作成される
Then 返却される `job_id` は `[0-9a-f]` のみで構成される固定長文字列である
And `job_id` は `01` 始まりの ULID 形式を前提にしていない
And `<root>/<job_id>/` が作成される

#### Scenario: create creates a hash-like job ID

Given `agent-exec create -- echo hi` を実行する
When ジョブが作成される
Then 返却される `job_id` は `[0-9a-f]` のみで構成される固定長文字列である
And `meta.json.job.id` はその完全 ID と一致する

### Requirement: short job ID in list output

`list` の各 job summary は完全な `job_id` に加えて、完全 ID の先頭 7 文字を表す `short_job_id` を含まなければならない（MUST）。`short_job_id` は人間向け表示用の省略表現であり、canonical identifier の代替ではない（MUST）。

#### Scenario: list returns short job IDs

Given 少なくとも 1 件の job が存在する
When `agent-exec list` を実行する
Then `jobs` の各要素は `job_id` と `short_job_id` を含む
And `short_job_id` は `job_id` の先頭 7 文字と一致する

## MODIFIED Requirements

### Requirement: list の JSON ペイロード

`list` は `root`, `jobs`, `truncated`, `skipped` を含む JSON を返さなければならない（MUST）。`jobs` の各要素は少なくとも `job_id`, `short_job_id`, `state`, `started_at` を含み、`exit_code` と `finished_at` と `updated_at` は存在する場合にのみ含めてよい（MAY）。

#### Scenario: list が必須フィールドを返す

Given `agent-exec list` を実行する
When コマンドが完了する
Then JSON に `root`, `jobs`, `truncated`, `skipped` が含まれる
And `jobs` の各要素は `job_id`, `short_job_id`, `state`, `started_at` を含む

### Requirement: create and start lifecycle commands

`agent-exec` MUST support a two-step lifecycle in addition to immediate `run`. `create` MUST persist a job definition without launching the command, and `start <job_id>` MUST launch a previously created job using the persisted definition. job lookup は完全一致または一意な先頭 prefix による指定を受け付けなければならない（MUST）。完全一致が存在しない場合に prefix 一致候補が 1 件だけなら、その job を解決しなければならない（MUST）。prefix 一致候補が複数ある場合は `ambiguous_job_id` を返さなければならない（MUST）。

#### Scenario: unique prefix resolves a hash-like job

Given hash-like `job_id` を持つ job が存在する
And その先頭 prefix が root 配下で一意である
When `agent-exec status <prefix>` を実行する
Then コマンドは対応する完全 `job_id` の job を解決して成功する

#### Scenario: ambiguous prefix is rejected

Given 2 件の job が同じ先頭 prefix を共有して存在する
When `agent-exec status <shared-prefix>` を実行する
Then コマンドは `ambiguous_job_id` を返して失敗する

### Requirement: backward compatibility for existing jobs

job lookup は既存の ULID 形式 job directory と新しい hash-like job directory の両方を扱えなければならない（MUST）。新規生成は新形式へ移行してよいが、既存 job の読み取り・待機・停止・削除・開始互換は維持しなければならない（MUST）。

#### Scenario: existing ULID jobs remain addressable

Given root 配下に既存 ULID 形式の job directory が存在する
When `agent-exec status <ulid-job-id>` を実行する
Then コマンドはその job を解決して状態を返す
And 新形式 job の導入によって既存 ULID job の参照は壊れない
