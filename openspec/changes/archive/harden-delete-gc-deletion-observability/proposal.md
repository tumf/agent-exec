---
change_type: implementation
priority: high
dependencies: []
references:
  - src/delete.rs
  - src/gc.rs
  - src/jobstore.rs
  - src/main.rs
  - tests/integration.rs
  - README.md
  - openspec/specs/agent-exec/spec.md
---

# 変更提案: harden-delete-gc-deletion-observability

**Change Type**: implementation

## Premise / Context
- 現セッションでは「`gc` や `delete` で消されたはずの job が `~/.local/share/agent-exec/jobs` に残る」という運用上の混乱が報告された。
- 現行実装では `delete <job_id>` は `running` 以外を即時削除し、`delete --all` は `meta.json.cwd` が現在の cwd と一致する terminal job だけを対象にする (`src/delete.rs`)。
- 現行 `gc` は root 全体を走査するが、`running`、`too_recent`、`state_unreadable` などは skip され、削除対象は age 条件を満たす terminal job に限られる (`src/gc.rs`)。
- 両コマンドとも `remove_dir_all` 成功後の事後確認をしておらず、削除対象外 job と削除失敗/再生成 job を利用者がレスポンスだけで区別しにくい。
- canonical spec には `list` の cwd filter や root 解決規則はあるが、`delete` / `gc` の「削除成功の事後保証」や「有効スコープの可視化」は明文化されていない。

## Requested Artifact
- implementation

## 背景 / 課題
`delete` と `gc` はどちらも job directory を削除するが、利用者から見たときの「何を対象にしたか」「本当に消えたのか」が十分に観測できない。

特に `delete --all` は cwd スコープ、`gc` は retention window と state 条件を前提にしているため、レスポンスの読み解きに失敗すると「削除したのに残っている」と感じやすい。また、現行コードは `remove_dir_all` が成功したら即 `deleted` とみなすため、削除後に path が再び存在するケースや root の取り違えを診断しにくい。

これは削除ロジックそのものだけではなく、削除結果の信頼性と scope observability の不足による運用バグである。

## 提案する変更
- `delete` と `gc` の削除経路に post-delete existence check を追加し、`remove_dir_all` 成功後も対象 path が存在する場合は `deleted` ではなく failure/skip として返す。
- `delete --all` レスポンスに有効 scope（少なくとも effective cwd）を含め、どの cwd に対して bulk delete を評価したかを明示する。
- `gc` / `delete` のレスポンスに、削除対象外・スキップ・削除成功を切り分けやすい集計または reason を追加し、利用者が「対象外だっただけ」なのか「削除できなかった」のかを判断できるようにする。
- README と spec を更新し、`delete --all` は cwd-scoped、`gc` は root-wide retention-based であること、ならびにレスポンスで scope と削除有効性を診断できることを明示する。
- integration tests を追加し、「deleted を返した job directory は同レスポンス時点で存在しない」こと、および scope 情報がレスポンスに含まれることを回帰防止として固定する。

## Acceptance Criteria
- `delete <job_id>` または `delete --all` が `jobs[].action="deleted"` を返す job は、同じ command invocation の完了時点で対象 job directory が存在しないことが検証される。
- `gc` が `jobs[].action="deleted"` を返す job についても、同じ command invocation の完了時点で対象 job directory が存在しないことが検証される。
- `delete --all` の JSON レスポンスから、どの cwd scope に対して削除評価したかを利用者が確認できる。
- `delete --all` と `gc` の JSON レスポンスから、少なくとも「削除成功」「対象内だが削除されなかった」「対象外/条件不一致」の区別が現状より明確になる。
- README と canonical spec に `delete` / `gc` の scope と post-delete observability 契約が反映される。
- strict validation が通り、実装時の検証計画に `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` が含まれる。

## Out of Scope
- `delete --all` の scope を cwd から root-wide に変更すること
- `gc` の retention window 既定値や terminal state 判定そのものの変更
- job root 解決優先順位 (`--root` / `AGENT_EXEC_ROOT` / `XDG_DATA_HOME`) の変更
