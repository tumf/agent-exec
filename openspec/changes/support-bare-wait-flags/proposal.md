---
change_type: implementation
priority: medium
dependencies: []
references:
  - AGENTS.md
  - src/main.rs
  - tests/integration.rs
  - README.md
  - skills/agent-exec/SKILL.md
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-run/spec.md
  - openspec/changes/archive/restore-inline-output-contract/proposal.md
---

# 変更提案: support-bare-wait-flags

**Change Type**: implementation

## Premise / Context
- このセッションでは、ユーザーが `run --wait` を指定したときに「真偽必須」と要求される現状を不自然だと問題提起している。
- 現行実装の clap surface では `run` / `start` の `wait` が `bool + ArgAction::Set` として定義されており、裸の `--wait` ではなく `--wait true|false` を要求する (`src/main.rs:204-218`, `src/main.rs:322-336`)。
- 既存統合テストも `run --wait true` / `start --wait true` を受理契約として固定している (`tests/integration.rs:1588-1601`)。
- 一方で canonical spec と README/skills は `--wait` を観測モードの正規フラグとして教えており、`--no-wait` もすでに存在するため、人間向け CLI としては裸の `--wait` を true 扱いできる余地がある (`openspec/specs/agent-exec/spec.md:189-212`, `openspec/specs/agent-exec-run/spec.md:420-438`, `README.md:170-196`, `skills/agent-exec/SKILL.md:16`)。
- このリポジトリでは CLI 契約変更時に `src/main.rs` と `tests/integration.rs`、必要な docs/spec を揃えることが求められる (`AGENTS.md:141-147`)。

## Requested Artifact
- implementation
- `run` / `start` の `--wait` を裸指定で true 扱いできるようにしつつ、`--no-wait` と `--until` / `--forever` の既存意味論を保つ。

## 背景 / 課題
現行の `run --wait` / `start --wait` は、フラグ名から期待される「指定したら有効化される」動作ではなく、`--wait true` または `--wait false` の明示入力を要求する。これは clap 実装都合としては一貫していても、人間が使う CLI としては不自然で、特に `--no-wait` が既に存在する現状では二重にわかりにくい。結果として、利用者は `--wait` を追加しただけで usage error に遭遇し、`wait` サブコマンドとの違いも含めて学習コストが上がる。

## 提案する変更
- `run` と `start` の `--wait` を裸指定で `true` 扱いできるように clap surface を変更する。
- 互換性のため、既存の `--wait true` / `--wait false` も引き続き受理する。
- `--no-wait` は引き続き `wait=false`, `until=0`, `forever=false` のエイリアスとして保持する。
- `--until` / `--forever` の既存排他・意味論は維持し、`--wait` の指定方法だけを人間向けに自然化する。
- README / skills / canonical spec / help 文言を更新し、`--wait` は裸で使えること、必要なら `--wait false` も後方互換で使えることを明記する。
- 統合テストを更新し、裸の `--wait` を主契約にしつつ、既存の明示 bool 形式も回帰テストとして残す。

## Acceptance Criteria
- `agent-exec run --wait -- echo hi` は usage error にならず、`--wait true` と同じ意味で成功する。
- `agent-exec start --wait <job_id>` は usage error にならず、`--wait true` と同じ意味で成功する。
- `agent-exec run --wait true -- echo hi` と `agent-exec start --wait true <job_id>` は後方互換として引き続き成功する。
- `agent-exec run --wait false -- echo hi` と `agent-exec start --wait false <job_id>` は引き続き追加待機なしの意味で受理される。
- `--no-wait`, `--until`, `--forever` の既存意味論および排他条件は変わらない。
- README / skills / canonical spec / clap help は、`--wait` を裸で使える主契約と整合している。

## Out of Scope
- `wait` サブコマンド自体の仕様変更
- `run` / `start` の inline output 契約や JSON shape の変更
- `serve` の JSON API で受け取る `wait: true|false` フィールドの仕様変更
