---
change_type: implementation
priority: high
dependencies: []
references:
  - src/jobstore.rs
  - src/schema.rs
  - src/main.rs
---

# 変更提案: ambiguous_job_id エラーに候補を構造化フィールドで返す

## Problem / Context

prefix 衝突時、`AmbiguousJobId` (`src/jobstore.rs:31-51`) は内部的に `candidates: Vec<String>` を持つが、`Display` で human-readable 文字列化されるだけ。`ErrorDetail` (`src/schema.rs:75-81`) は `code`/`message`/`retryable` のみで構造化候補が無い。`main.rs:692-693` で message に埋め込まれる。

結果、エージェントは候補を再取得するため `list` を追加で 1 往復、さらに prefix 絞り込みの判断にもう 1 往復と、2 往復目以降を強いられる。

## Proposed Solution

- `ErrorDetail` に `details: Option<serde_json::Value>` を追加。
- `ambiguous_job_id` エラー発生時、`details` に `{"candidates":[job_id...],"truncated":bool}` を構造化して入れる。
- 候補は最大 20 件、超過時 `truncated=true`（現行の 5 件+「and N more」より拡張）。
- 既存 `message` は human readable のまま保持。

## Acceptance Criteria

- `agent-exec status <ambiguous-prefix>` が構造化 `error.details.candidates` を含む JSON を返す。
- `POST /status/:id` 等 HTTP 経由でも同じ。
- `truncated=true` が 21 件以上で立つ。

## Out of Scope

- 他のエラーコード向け `details` 用途は本提案では扱わない（拡張枠のみ追加）。
