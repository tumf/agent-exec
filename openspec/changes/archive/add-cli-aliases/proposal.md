---
change_type: implementation
priority: medium
dependencies: []
references:
  - /Users/tumf/AGENTS.md
  - AGENTS.md
  - src/main.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
  - openspec/changes/archive/add-delete-command
  - openspec/changes/archive/add-job-list-subcommand
  - openspec/changes/archive/add-list-state-filter
---

# 変更提案: add-cli-aliases

**Change Type**: implementation

## Premise / Context
- このセッションでは、ユーザーが `agent-exec ps` を `agent-exec list --state running` の直感的 alias として使いたい、`agent-exec rm` を `delete` の alias にしたい、と要望している。
- 現行 CLI は `src/main.rs` の `clap::Subcommand` で `List` と `Delete` を個別定義しており、`ps` / `rm` の alias は未実装である。
- canonical spec には `list --state running` で実行中ジョブだけを返す契約があり、`delete` は finished job を削除する契約があるため、今回の変更は新しい動作追加ではなく既存操作への短い到達経路の追加である。
- リポジトリ方針として、CLI 契約変更時は `src/main.rs`、`tests/integration.rs`、関連 spec を同期する必要がある。
- ユーザーは追加候補のアイデアも求めていたが、今回の proposal は明示要求された `ps` と `rm` に絞り、他の alias 候補は out of scope として扱う。

## Requested Artifact
- implementation
- `agent-exec ps` を `list --state running` の短縮形として追加する。
- `agent-exec rm` を `delete` の alias として追加する。

## 背景 / 課題
`agent-exec` は job 管理 CLI だが、日常的に使う「実行中 job を見る」「job を削除する」という操作は `list --state running` と `delete` のように少し長い。既存機能だけでも目的は達成できるが、エージェントや人間が短いコマンドで到達できないため、反復操作時の認知負荷が高い。特に `list --state running` は意味的には Unix 的な `ps` に近く、`delete` は `rm` に相当するため、覚えやすい alias を持たせる価値がある。

## 提案する変更
- `ps` サブコマンドを追加し、`list` 実装へ `state=running` を固定して委譲する。
- `ps` は `list` と同じ絞り込みオプションのうち `--limit` / `--cwd` / `--all` / `--tag` を受け付けるが、`--state` は露出しない。
- `rm` は `delete` の visible alias として実装し、既存の `delete <JOB_ID>` / `delete --all` / `delete --dry-run` 契約をそのまま使う。
- JSON envelope や command semantics は既存の `list` / `delete` から変更せず、短い別名だけを追加する。
- integration tests と canonical spec を更新し、`ps` と `rm` が既存契約の別到達経路であることを固定する。

## Acceptance Criteria
- `agent-exec ps` は `agent-exec list --state running` と同じ running job だけを返す。
- `agent-exec ps --all` / `agent-exec ps --cwd <PATH>` / `agent-exec ps --tag <PATTERN>` / `agent-exec ps --limit <N>` は、それぞれ `list --state running` に同じオプションを付けた場合と同等に振る舞う。
- `agent-exec ps` は `--state` を新たに露出せず、running 固定の短縮形として振る舞う。
- `agent-exec rm <JOB_ID>` は `agent-exec delete <JOB_ID>` と同じ削除結果を返す。
- `agent-exec rm --all` と `agent-exec rm --dry-run --all` は `delete` と同じ bulk delete 契約を保つ。
- README/spec/test が、`ps` と `rm` が既存機能の alias / shorthand であることと整合している。

## Out of Scope
- `ls` / `logs` / `stop` など、今回ユーザーが明示要求していない追加 alias の実装
- `list` / `delete` 自体の JSON shape や削除条件の変更
- `serve` API における HTTP route alias の追加
