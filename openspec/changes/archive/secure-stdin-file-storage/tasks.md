## Implementation Tasks

- [x] Task 1: Unix で `stdin.bin` を `OpenOptions::new().mode(0o600)` で作成（`#[cfg(unix)]` 分岐）。verification: unit — `src/run.rs`
- [x] Task 2: 書き込み時にサイズカウンタを保持し、64 MiB 超過で `anyhow::bail!` → `stdin_too_large`。verification: unit
- [x] Task 3: CLI に `--stdin-max-bytes` を追加（既定 67108864）。verification: unit — clap
- [x] Task 4: integration test (cfg(unix)): `stdin.bin` のパーミッションが 0o600 であることを確認。verification: integration — `tests/integration.rs`
- [x] Task 5: integration test: 65 MiB 超入力で `stdin_too_large`。verification: integration

## Specification Tasks

- [x] Promote `specs/agent-exec-jobstore/spec.md` delta — stdin.bin の保存仕様 MUST 化

## Future Work

- Windows ACL 設計は別提案。
