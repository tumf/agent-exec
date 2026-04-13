---
change_type: spec-only
priority: low
dependencies: []
references:
  - src/schema.rs
  - src/jobstore.rs
  - openspec/specs/agent-exec-windows/spec.md
---

# 変更提案: Windows state.json の windows_job_name を canonical 化

## Problem / Context

実装は Windows 上で `JobState.windows_job_name: Option<String>` に `"AgentExec-{job_id}"` を書く (`src/schema.rs:658-663`, `src/jobstore.rs:399-423`)。`pid` は共通。canonical spec は「Job Object を識別できる情報」とだけあり具体フィールド名未定義。

## Requested Artifact: spec/documentation

実装の canonical 化。

## Proposed Solution

- `agent-exec-windows/spec.md` に MUST で記載:
  - Windows の `state.json` に `windows_job_name: string` を含める。
  - 形式は `AgentExec-{job_id}`。
  - 他プラットフォームでは `windows_job_name` は省略または null。

## Acceptance Criteria

- canonical spec にフィールド名と形式が記載される。

## Out of Scope

- 実装変更。
