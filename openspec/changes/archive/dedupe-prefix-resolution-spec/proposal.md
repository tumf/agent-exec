---
change_type: spec-only
priority: low
dependencies: []
references:
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-serve/spec.md
  - src/jobstore.rs
---

# 変更提案: prefix 解決 spec を agent-exec に一本化し serve は参照のみにする

## Problem / Context

prefix 解決規則が CLI 側 `agent-exec/spec.md` と HTTP 側 `agent-exec-serve/spec.md` に重複定義されている（`agent-exec-serve/spec.md` L142-157 / `agent-exec/spec.md` の prefix セクション）。実装は `JobDir::open` (`src/jobstore.rs:169-212`) に集約されており、CLI も serve も同じ関数を呼ぶので片方の spec 更新忘れで drift する。

## Requested Artifact: spec/documentation

spec の整理のみ。

## Proposed Solution

- canonical prefix 解決 Requirement を `agent-exec/spec.md` のみに置く。
- `agent-exec-serve/spec.md` の対応セクションを削除し「`GET /status/:id`・`GET /tail/:id`・`GET /wait/:id`・`POST /kill/:id` は CLI の `status`/`tail` 等と同じ prefix 解決規則に従う」の 1 行参照に置き換える。

## Acceptance Criteria

- `openspec validate --specs --strict` が通る（pre-existing Purpose 欠落以外のエラーが発生しない）。
- spec grep で prefix 解決規則が canonical 1 箇所のみにヒットする。

## Out of Scope

- 実装変更。
