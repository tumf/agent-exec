## Implementation Tasks

- [ ] Task 1: `generate_job_id` の衝突 loop に 16 回の上限を導入、超過時 `anyhow::bail!` で `io_error`
  - verification: unit — `src/jobstore.rs`
- [ ] Task 2: `main.rs` のエラーマッピングで `io_error` code を返す
  - verification: unit
- [ ] Task 3: unit test: 乱数を差し替えて 16 連続衝突をシミュレートし失敗することを確認
  - verification: unit

## Specification Tasks

- [ ] Promote `specs/agent-exec-jobstore/spec.md` delta — job_id 生成仕様
  - Expected canonical result: `32 文字小文字 hex`・`128bit CSPRNG`・`衝突時 16 回リトライ` を MUST 明記

## Future Work

- なし
