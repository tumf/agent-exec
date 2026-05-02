## MODIFIED Requirements

### Requirement: ジョブディレクトリ構造

各ジョブは `<root>/<job_id>/` に作成し、`meta.json`, `state.json`, `stdout.log`, `stderr.log`, `full.log` を含まなければならない（MUST）。新規 job に対する `job_id` directory 名は小文字 hex ベースの hash-like ID でなければならない（MUST）。既存 ULID directory は互換のため引き続き開けなければならない（MUST）。

jobs root の整理機能は、この `<root>/<job_id>/` layout を維持したまま動作しなければならない（MUST）。auto-GC または manual GC の導入によって既存 job の完全 ID / prefix lookup / ULID 互換を壊してはならない（MUST NOT）。

#### Scenario: new jobs use hash-like directory names

Given `agent-exec run -- <cmd>` を実行する
When ジョブが作成される
Then job directory 名は返却された完全 `job_id` と一致する
And その directory 名は `[0-9a-f]` のみで構成される固定長文字列である

#### Scenario: cleanup preserves flat job directory layout

**Given**: cleanup 対象外の existing job directory が `<root>/<job_id>/` に存在する
**When**: `agent-exec gc`, `agent-exec run`, または `agent-exec start` による cleanup 評価が実行される
**Then**: 対象外 job directory は別階層へ移動されない
**And**: その job は従来通り完全 ID または一意 prefix で参照できる

#### Scenario: cleanup keeps legacy ULID job lookup compatible

**Given**: root 配下に既存 ULID 形式の job directory が存在する
**When**: cleanup 評価後に `agent-exec status <ulid-job-id>` を実行する
**Then**: cleanup 対象外の ULID job は引き続き解決される
