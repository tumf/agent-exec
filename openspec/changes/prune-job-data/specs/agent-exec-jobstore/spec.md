# agent-exec-jobstore Specification (Change: prune-job-data)

## ADDED Requirements

### Requirement: terminal job の age-based GC

GC は `state.json.job.status` が `exited|killed|failed` の job だけを削除対象として評価しなければならない（MUST）。`running` job は削除対象にしてはならない（MUST）。削除判定に使う時刻は `finished_at` を優先し、`finished_at` が無い場合のみ `updated_at` を使用してよい（MAY）。両方が無い job は安全側でスキップしなければならない（MUST）。

#### Scenario: running job は保持される
Given 実行中 job の `state.json.job.status` が `running` である
When `agent-exec gc --older-than 7d` を実行する
Then その job ディレクトリは削除されない

#### Scenario: finished_at で削除判定する
Given `state.json.job.status=exited` かつ `finished_at` が 7 日より古い terminal job がある
When `agent-exec gc --older-than 7d` を実行する
Then その job ディレクトリは削除対象になる

#### Scenario: updated_at へフォールバックする
Given `state.json.job.status=failed` で `finished_at` が欠落し、`updated_at` が 7 日より古い job がある
When `agent-exec gc --older-than 7d` を実行する
Then その job ディレクトリは削除対象になる

#### Scenario: 判定時刻が無い job はスキップする
Given `state.json.job.status=killed` だが `finished_at` と `updated_at` の両方が無い job がある
When `agent-exec gc --older-than 7d` を実行する
Then その job ディレクトリは削除されない
And JSON レスポンスでは skip 理由を観測できる

### Requirement: gc は root 全体を対象にする

GC は resolved root 配下の job ディレクトリ全体を対象にしなければならない（MUST）。`list` の cwd フィルタ条件を暗黙に適用してはならない（MUST）。

#### Scenario: cwd に依存せず走査する
Given root 配下に複数の `cwd` 由来の terminal job が存在する
When `agent-exec gc --older-than 7d` を実行する
Then 削除対象の評価は caller の current_dir に依存しない

### Requirement: job directory の再帰削除

GC が job を削除する場合、`<root>/<job_id>/` ディレクトリ全体を再帰削除しなければならない（MUST）。削除前に job directory の総バイト数を計算し、レスポンスの `freed_bytes` に反映しなければならない（MUST）。

#### Scenario: job directory 全体を削除する
Given terminal job directory に `meta.json`, `state.json`, `stdout.log`, `stderr.log`, `full.log`, `completion_event.json` が存在する
When `agent-exec gc --older-than 7d` を実行する
Then `<root>/<job_id>/` は丸ごと削除される
And `freed_bytes` はそのディレクトリの削除前サイズ以上である
