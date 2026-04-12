---
change_type: implementation
priority: high
dependencies: []
references:
  - openspec/specs/agent-exec-run/spec.md
  - openspec/specs/agent-exec-tests/spec.md
  - src/main.rs
  - tests/integration.rs
  - README.md
  - skills/agent-exec/SKILL.md
---

# 変更提案: remove-wait-timeout-ms-alias

**Change Type**: implementation

## Premise / Context
- 現セッションでは `agent-exec wait --timeout-ms 900000` の解釈をめぐって、仕様と実装が食い違っていることが確認された。
- canonical spec は `wait` の待機期限を `--until` に統一し、既存の `--timeout-ms` を置換済みとしている (`openspec/specs/agent-exec-run/spec.md:285`-`289`)。
- 一方で現行実装は `src/main.rs` で `--timeout-ms` を `--until` の alias として受理し、`tests/integration.rs`・`README.md`・`skills/agent-exec/SKILL.md` にも旧語彙が残っている。
- ユーザ要求は「明確化済みの仕様があるのに、ユニット/統合テストやスキルがそれを反映していないのはおかしい。スキルも含めて修正したい」というもの。
- リポジトリ指針では CLI 契約変更時に `src/main.rs` と `tests/integration.rs` を更新し、`cargo fmt` / `cargo clippy` / `cargo test` または `prek run -a` による検証を行う必要がある (`AGENTS.md:141`-`147`)。

## Requested Artifact
- implementation

## 背景 / 課題
`wait` サブコマンドの待機期限は canonical spec 上すでに `--until` に一本化されているが、実装とユーザ向け資産の一部は「deprecated alias をまだ受け付ける」前提のまま残っている。その結果、仕様を正とした利用者が `--timeout-ms` を無効オプションと期待しても、実際の CLI は 15 分待機として処理してしまう。

さらに、仕様の明確化後も統合テストが旧 alias の互換維持を前提にしており、README と埋め込みスキルも legacy 挙動を案内しているため、将来の実装修正やレビューで「どちらが正か」を再び混同しやすい。これは単なる文言のズレではなく、CLI 契約・テスト・スキル配布物が canonical spec に追随していないバグである。

## 提案する変更
- `wait` サブコマンドから `--timeout-ms` alias を削除し、待機期限指定を `--until` のみにする。
- `tests/integration.rs` の `wait` 関連テストを canonical spec 準拠に更新し、`--timeout-ms` は usage error になることを明示的に検証する。
- `README.md` と `skills/agent-exec/SKILL.md` を更新し、待機期限の正規語彙を `--until` に統一する。
- OpenSpec 側にも「仕様が明確化された後はテストと配布スキルが canonical spec に追随しなければならない」ことを追加し、再発防止の検証責務を明確にする。

## Acceptance Criteria
- `agent-exec wait --timeout-ms 100 <job_id>` は clap usage error として拒否される。
- `agent-exec wait --until 100 <job_id>`、`agent-exec wait <job_id>`、`agent-exec wait --forever <job_id>` の canonical spec 既存意味論は維持される。
- 統合テストは `wait` の正規待機期限語彙として `--until` のみを使用し、legacy alias を正規経路として前提にしない。
- `README.md` と `skills/agent-exec/SKILL.md` に `wait --timeout-ms` を正規例または互換案内として残さない。
- strict validation が通る proposal とし、実装時の検証計画に `cargo fmt --all`・`cargo clippy --all-targets --all-features -- -D warnings`・`cargo test --all` または `prek run -a` が含まれる。

## Out of Scope
- `run --timeout` や `create --timeout` のジョブ実行タイムアウト意味論の変更
- HTTP `GET /wait/{id}` の API 形状変更
- `wait` の既定 30,000ms 待機ロジックそのものの変更
