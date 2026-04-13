---
change_type: spec-only
priority: medium
dependencies: []
references:
  - src/schema.rs
  - src/list.rs
  - openspec/specs/agent-exec/spec.md
---

# 変更提案: list の進捗フィールドを state.json 存在時に MUST 化

## Problem / Context

`JobSummary` (`src/schema.rs:230-251`) は `updated_at` / `finished_at` / `exit_code` を Option + `skip_serializing_if` で保持。state.json があっても spec 上は MAY 扱いのため、エージェントは list 一発で進捗判断できない場合があり、`status` の 2 往復目を招く。

実装 (`src/list.rs:182-206`) は state.json が読めれば埋めるため、**コード追加は不要**。canonical spec を MUST に引き上げるだけでコンセプト整合する。

## Requested Artifact: spec/documentation

本提案は spec のみの改定で、実装は既存の挙動を canonical 化するだけ。

## Proposed Solution

- `agent-exec/spec.md` の `list の JSON ペイロード` 項に「state.json が存在する job については `updated_at`・`finished_at`（終端時）・`exit_code`（終端時）を必ず含める MUST」を追記。
- state.json が race 条件で読めない場合のみ省略可とする。

## Acceptance Criteria

- canonical spec に進捗フィールドの MUST 化が記載される。
- 既存 integration test が仕様改定後も通ることを確認（実装変更不要）。

## Out of Scope

- 新規フィールド追加なし。
