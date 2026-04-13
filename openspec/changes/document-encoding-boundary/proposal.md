---
change_type: spec-only
priority: low
dependencies: []
references:
  - src/jobstore.rs
  - openspec/specs/agent-exec-run/spec.md
---

# 変更提案: max-bytes がマルチバイト境界をまたぐ場合の encoding 契約を明文化

## Problem / Context

`read_head_metrics` / `read_tail_metrics` (`src/jobstore.rs:322-360`) は `data[..included_len]` を `from_utf8_lossy` で文字列化し、マルチバイト文字の途中で切れた場合 U+FFFD に置換する。`encoding="utf-8-lossy"` とは付くが、「range はバイト単位」「切れたバイトは U+FFFD」と canonical spec で明示されていないため、`stdout_range` を使った再構築を試みる client が混乱する。

## Requested Artifact: spec/documentation

canonical 化。

## Proposed Solution

- `agent-exec-run/spec.md` に MUST で記載:
  - `stdout_range` / `stdout_total_bytes` はバイト単位。
  - `stdout` 文字列は範囲内バイト列を `from_utf8_lossy` 変換した結果。
  - マルチバイト文字の途中で範囲が切れた場合、そのバイト列は U+FFFD（3 バイト）として置換され、置換後の文字列バイト長と `range` の値は一致しない。
  - `encoding="utf-8-lossy"` が付与されている間は U+FFFD の出現はエンコーディング変換起因の可能性がある。

## Acceptance Criteria

- canonical spec に境界挙動が明記される。

## Out of Scope

- 実装変更。
