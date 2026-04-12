---
change_type: implementation
priority: high
dependencies: []
references:
  - AGENTS.md
  - src/main.rs
  - src/run.rs
  - src/create.rs
  - src/schema.rs
  - README.md
  - skills/agent-exec/SKILL.md
  - skills/agent-exec/references/cli-contract.md
  - skills/agent-exec/references/hermes.md
  - skills/agent-exec/references/openclaw.md
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-run/spec.md
  - tests/integration.rs
---

# 変更提案: standardize-runtime-time-seconds

**Change Type**: implementation

## Premise / Context
- 現セッションでは、`wait --until` / `--poll` に続いて `--timeout` など残存する ms ベース時間指定もすべて秒へ統一したい、という要求が明示された。
- 既存実装では `wait` はすでに秒ベースに移行している一方、`run` / `create` / `supervise` の `--timeout`、`--kill-after`、`--progress-every` には ms ベースの help・README・skill 記述が残っている (`src/main.rs`, `README.md`, `skills/agent-exec/**`)。
- さらに `snapshot-after` は削除済みのはずなのに、README/skill reference には一部旧記述が残存していたため、CLI 表面・仕様・配布スキルの不整合が再発している。
- canonical spec も `openspec/specs/agent-exec-run/spec.md` に旧 `snapshot-after` 中心の要件が残っており、`openspec/specs/agent-exec/spec.md` の「run は即時返却・snapshot 系を拒否する」要件と衝突している。
- リポジトリ指針では CLI 契約変更時に clap surface、spec、統合テストを一緒に更新し、`prek run -a` 相当で検証する必要がある (`AGENTS.md:141`-`147`)。

## Requested Artifact
- implementation

## 背景 / 課題
時間指定の単位がコマンドごとに混在していると、人間向け CLI として直感に反する。`wait` は秒、`run --timeout` は ms、古い `snapshot-after` 記述は削除済みなのに docs に残る、という状態では、利用者が help・README・skill を読んでも正しい契約を即座に把握できない。特に `--timeout` と `--kill-after` は人が直接指定するオプションであり、`30000` より `30` のほうが自然で誤設定も起きにくい。

同時に、`snapshot-after` の削除が canonical spec やスキル文書へ十分に反映されておらず、現在の実装・docs・spec の整合性が崩れている。これは単なる文言調整ではなく、CLI 契約の正規ソースを再統一する作業である。

## 提案する変更
- `run` / `create` / 必要な内部 supervise surface に露出する人間向け時間オプション `--timeout`、`--kill-after`、`--progress-every` を秒単位へ統一する。
- 内部実装は必要に応じてミリ秒へ変換してよいが、clap help、README、skill、examples、integration tests は秒単位を正規表現とする。
- `snapshot-after` 削除済みの契約を canonical spec、README、skills references に反映し、現行表面 API から除かれたオプションを正規例として残さない。
- `openspec/specs/agent-exec-run/spec.md` に残る旧 snapshot-centered requirement を現行 `agent-exec/spec.md` の「即時返却 + wait/tail 分離」契約へ整合させる。
- 旧 ms 解釈や削除済み snapshot 系フラグを残す場合は、その互換/拒否挙動を明示的に spec と integration tests で定義する。

## Acceptance Criteria
- `--timeout`、`--kill-after`、`--progress-every` は人間向け CLI 契約上すべて秒単位として案内される。
- `src/main.rs` の clap help、`README.md`、`skills/agent-exec/**` は上記時間オプションを秒単位で一貫して説明する。
- canonical spec は `run` の snapshot-centered 旧要件を残さず、現行の即時返却・削除済み `snapshot-after` 拒否・観測責務分離と整合する。
- 統合テストは秒単位の `timeout` / `kill-after` / `progress-every` の契約と、削除済み `snapshot-after` の拒否挙動を正規経路として検証する。
- 実装時の検証計画に `cargo fmt --all`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` または `prek run -a` が含まれる。

## Out of Scope
- 内部保存メトリクス `elapsed_ms` や `duration_ms` など JSON/schema の内部単位名変更
- 過去 archive proposal 全件の文言一括書き換え
- 秒未満の厳密な runtime scheduling 保証
