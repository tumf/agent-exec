---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/list.rs
  - src/schema.rs
---

# 変更提案: list --limit の既定値を 50 に引き下げる

## Problem / Context

`list --limit` の既定は `0`（無制限、`src/main.rs:457-458`）。cwd filter が効いていても大作業ツリーでは結果が膨らみ、agent の LLM コンテキストを圧迫する。`ListData.truncated` (`src/schema.rs:268-269`) は実装済みだが既定無制限のため発動しない。

## Proposed Solution

- `--limit` の既定を `50` に変更（既存 `0` は引き続き「明示的無制限」として受理）。
- spec: canonical 値を 50 で MUST 化。truncated=true で「続きがあることを agent が 1 回の応答で判定できる」ことを明記。

## Acceptance Criteria

- `agent-exec list` が既定で最大 50 件返し、それ以上あれば `truncated=true`。
- `agent-exec list --limit 0` で従来通り全件。
- `agent-exec list --limit N` で任意件数。

## Out of Scope

- ページネーション token（offset / cursor）の追加は本提案に含めない。
