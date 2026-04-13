## Implementation Tasks

- [x] Task 1: `src/main.rs:457-458` の `--limit` 既定値を `0` から `50` に変更
  - verification: unit — clap default テスト
- [x] Task 2: integration test: 60 件 job を作り既定 `list` で 50 件 + `truncated=true` を確認
  - verification: integration — `tests/integration.rs`
- [x] Task 3: integration test: `--limit 0` で全件返ることを確認
  - verification: integration

## Future Work

- ページネーション token は別提案。
