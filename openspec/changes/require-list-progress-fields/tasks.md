## Specification Tasks

- [ ] Promote `specs/agent-exec/spec.md` delta — list 進捗フィールド MUST 化
  - Expected canonical result: `list` の各エントリは state.json が読める場合 `updated_at` を必ず含み、終端到達時は `exit_code`・`finished_at` も必ず含む
- [ ] Review delta scenarios for coverage (running / finished / state-missing の 3 ケース)

## Future Work

- 実装側の MUST 違反を検知する integration test 追加は `enrich-run-inline-completion` などと合わせて別途検討（本提案は canonical 化のみ）。
