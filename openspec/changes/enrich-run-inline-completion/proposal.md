---
change_type: implementation
priority: high
dependencies: []
references:
  - src/schema.rs
  - src/run.rs
  - openspec/specs/agent-exec-run/spec.md
---

# 変更提案: run/start 初回 inline レスポンスに signal / duration_ms を追加

## Problem / Context

コアコンセプトは「起動 + 早期失敗観測を 1 往復で終わらせる」こと。`run` / `start` の既定 10 秒観測内で短命ジョブが終了した場合、現状 `RunData` (`src/schema.rs:124-163`) は `exit_code` / `finished_at` のみを返し、`signal` と `duration_ms` フィールド自体が存在しない。`observe_inline_output` (`src/run.rs:642-658`) も state.json から exit_code / finished_at を抽出するのみ。

その結果、エージェントは「signal 起因の異常終了か」「どれだけ時間を使ったか」を知るために `status` を叩く 2 往復目を強いられており、コアコンセプトに反する。

## Proposed Solution

- `RunData` に `signal: Option<String>`（signal 終了時のみ）と `duration_ms: Option<u64>`（`finished_at - started_at`）を追加。
- `observe_inline_output` が state.json から signal / started_at を読み、終端到達時に両フィールドを埋める。
- spec では「10 秒以内に終端状態へ到達した場合、`exit_code`・`finished_at`・`signal`（signal 終了時）・`duration_ms` を必ず含める」を MUST 化。

## Acceptance Criteria

- `run` で `sh -c "exit 7"` を走らせると `exit_code=7` / `finished_at` / `duration_ms` を含む JSON が返る。
- `run` で `sh -c "kill -TERM $$"` を走らせると `signal` が含まれる。
- 10 秒以内に終わらなかったジョブでは `signal` / `duration_ms` / `exit_code` / `finished_at` がいずれも省略される。
- 契約違反（終端到達したのに exit_code 欠落など）が発生しない integration test が存在する。

## Out of Scope

- `status` / `tail` / `wait` 系の同フィールド追加は別提案で扱う（本提案は run/start の inline 観測のみ）。
