---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/kill.rs
  - src/serve.rs
  - src/schema.rs
---

# 変更提案: kill レスポンスに post-signal 観測を含める

## Problem / Context

`KillData` (`src/schema.rs:212-216`) は `job_id` / `signal` のみ。CLI `kill` (`src/kill.rs:100-110`) と HTTP `POST /kill/:id` (`src/serve.rs:475-482`) のいずれも signal 送信結果の echo のみで、エージェントは「本当に死んだか」確認のために `status` 往復を強いられる。

## Proposed Solution

- `KillData` に `state`・`exit_code: Option<i32>`・`terminated_signal: Option<String>`・`observed_within_ms: Option<u64>` を追加。
- `kill` 実装で signal 送信後に短時間（既定 3秒）の inline wait を行い、state の遷移を観測して埋める。
- `--no-wait` フラグで従来の即時返却を選択可能に。
- spec: `kill` は既定で post-signal 観測を MUST、`--no-wait` でオプトアウト可。

## Acceptance Criteria

- `agent-exec kill <running-job>` が既定で終端 `state` と `exit_code` を返す。
- `--no-wait` 指定時は既存 shape（`job_id` / `signal` のみ）が返る。
- 3秒以内に終わらなかった場合は `state=running` のまま返り、`observed_within_ms=3000` を付与。

## Out of Scope

- `kill` の grace period（`--kill-after` の再定義）は別提案。
