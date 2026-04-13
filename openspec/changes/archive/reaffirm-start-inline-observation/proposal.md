---
change_type: spec-only
priority: low
dependencies: []
references:
  - src/start.rs
  - src/main.rs
  - openspec/specs/agent-exec/spec.md
---

# 変更提案: start の既定 inline 観測を agent-exec spec で再掲する

## Problem / Context

実装は `start --wait` default true / `--until` default 10 (`src/main.rs:208-219`, `src/start.rs:103-109`) でコンセプト通り。しかし canonical `agent-exec/spec.md` 側の `create` / `start` 関連記述が薄く、実装が将来 launch-only に流れるリスクがある（矛盾する旧仕様を掃除した際も当該セクションが曖昧なまま）。

## Requested Artifact: spec/documentation

再掲のみ。

## Proposed Solution

- `agent-exec/spec.md` の create/start ライフサイクル Requirement に「`start` の既定 inline 観測は `run` と同一契約（`--wait` 既定 true、`--until` 既定 10、`--max-bytes` 既定 65536、inline output field と終端フィールド）」を MUST として明示再掲。
- `agent-exec-run/spec.md` 側との参照関係を明確にする。

## Acceptance Criteria

- canonical spec 読解で `start` が launch-only に誤実装される余地が無くなる。

## Out of Scope

- 実装変更。
