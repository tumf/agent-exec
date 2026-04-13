---
change_type: spec-only
priority: low
dependencies: []
references:
  - src/jobstore.rs
  - openspec/specs/agent-exec-jobstore/spec.md
---

# 変更提案: ULID 互換の読み取り挙動を canonical 化

## Problem / Context

新規生成は hex のみ（`src/jobstore.rs:69-87`）だが、`JobDir::open` は形式を問わず文字列比較するため過去の ULID 風 job (26 文字) も読める（テスト `src/jobstore.rs:867-883` で確認済み）。spec に明文化されておらず、「ULID の `0` 始まりは hex としても解釈可能」といったエッジケースでどちらに解決するか不明。

## Requested Artifact: spec/documentation

canonical 化。

## Proposed Solution

- `agent-exec-jobstore/spec.md` に以下を MUST で明記:
  - 新規生成は 32 文字 hex のみ。
  - 既存ディレクトリは形式を問わず `0-9a-zA-Z` の範囲で一致する prefix を許容し、文字列 prefix 比較で解決。
  - hex と ULID が併存する環境で両方にマッチする prefix が存在する場合、両方を `ambiguous_job_id` の candidates に含める（ULID 優先などの暗黙順位を付けない）。

## Acceptance Criteria

- canonical spec に読み取り互換契約が記載される。

## Out of Scope

- ULID 生成の復活。
