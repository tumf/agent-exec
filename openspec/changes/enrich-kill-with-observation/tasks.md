## Implementation Tasks

- [ ] Task 1: `KillData` に `state`・`exit_code`・`terminated_signal`・`observed_within_ms` を追加
  - verification: unit — `src/schema.rs`
- [ ] Task 2: `src/kill.rs` で signal 送信後に `observe_inline_output` 相当（本文不要、state のみ）で最大 3秒観測して埋める
  - verification: unit — kill テスト
- [ ] Task 3: CLI `kill` に `--no-wait` フラグ追加（既定 false、指定時は従来挙動）
  - verification: unit — clap parse テスト
- [ ] Task 4: `POST /kill/:id` も同じ観測を行う。`?no_wait=true` で opt-out 可
  - verification: integration — serve テスト
- [ ] Task 5: integration test: 通常 kill で terminal state と exit_code が返ることを検証
  - verification: integration
- [ ] Task 6: integration test: `--no-wait` で既存 shape が返ることを検証
  - verification: integration

## Future Work

- なし
