---
change_type: spec-only
priority: medium
dependencies: []
references:
  - src/main.rs
  - openspec/specs/agent-exec-run/spec.md
---

# 変更提案: run/start inline output の既定 max-bytes を 64 KiB で明文化

## Problem / Context

`--max-bytes` 既定値は実装上 `65536` (`src/main.rs:231`/`356`) だが、canonical spec に記載が無い。agent が LLM コンテキスト予算を組む際に保証された上限が不明で、巨大 stdout ジョブでコンテキスト破壊リスクがある。

## Requested Artifact: spec/documentation

実装値の canonical 化のみ。値変更はしない。

## Proposed Solution

- `agent-exec-run/spec.md` の inline output 要件に「既定 max-bytes は 65536（64 KiB）」を MUST で明記。
- 値変更は `schema_version` の bump を伴う break change として扱う旨を記載。

## Acceptance Criteria

- canonical spec に既定値が明文化される。
- 既定値を変える場合の手続き（SemVer bump）が記述される。

## Out of Scope

- 既定値の変更そのもの。
