## Implementation Tasks

- [x] Task 1: `RunData` に `signal: Option<String>` と `duration_ms: Option<u64>` を追加し、`skip_serializing_if = "Option::is_none"` を適用する (verification: unit — `src/schema.rs` の serde シリアライズテスト)
- [x] Task 2: `observe_inline_output` (`src/run.rs:642-658`) が state.json から `signal` と `started_at` を読み、終端到達時に RunData へ埋める (verification: unit — `src/run.rs` の関数テスト)
- [x] Task 3: `duration_ms` は `finished_at - started_at` をミリ秒で算出する（RFC3339 diff） (verification: unit — 時刻差算出テスト)
- [x] Task 4: integration test: `sh -c "exit 7"` で `exit_code=7` / `finished_at` / `duration_ms` が返ることを検証 (verification: integration — `tests/integration.rs`)
- [x] Task 5: integration test: `sh -c "kill -TERM $$"` で `signal` フィールドが含まれることを検証（Unix のみ） (verification: integration — `tests/integration.rs` cfg(unix))
- [x] Task 6: integration test: `sh -c "sleep 30"` で 10 秒観測では `signal` / `duration_ms` / `exit_code` / `finished_at` が省略されることを検証 (verification: integration — `tests/integration.rs`)

## Future Work

- `WaitData` / `StatusData` 側の同フィールド追加は別提案（scope 分離）。
