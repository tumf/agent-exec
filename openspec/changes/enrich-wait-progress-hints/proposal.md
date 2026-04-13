---
change_type: implementation
priority: high
dependencies: []
references:
  - src/schema.rs
  - src/wait.rs
  - openspec/specs/agent-exec-run/spec.md
---

# 変更提案: wait 満期時に進捗ヒントを返す

## Problem / Context

`wait --until` が満期に到達したとき、`WaitData` (`src/schema.rs:202-209`) は `job_id` / `state` / `exit_code` のみを返す。エージェントは「どこまで進んだか」「最後の更新はいつか」を知るために直後に `tail` や `status` を叩く 2 往復目を強いられ、コアコンセプトに反する。

## Proposed Solution

- `WaitData` に `stdout_total_bytes: Option<u64>`・`stderr_total_bytes: Option<u64>`・`updated_at: Option<String>` を追加。
- `wait` 満期分岐 (`src/wait.rs:70-80`) と終端返却分岐の両方で、state.json / log メトリクスを読んで埋める。
- spec: `wait` レスポンスで state.json が読める場合、これら 3 フィールドを MUST 化。

## Acceptance Criteria

- 実行中のジョブに対して `wait --until 1` を発行すると、`state=running` とともに `stdout_total_bytes` / `updated_at` が返る。
- ジョブが終端到達した場合も `stdout_total_bytes` / `updated_at` が返る。
- state.json が存在しない（race 条件で）場合は欠落してよい（Option）。

## Out of Scope

- `wait` に inline `stdout` / `stderr` 本文を乗せる機能は本提案に含めない（bytes 数のみ）。
