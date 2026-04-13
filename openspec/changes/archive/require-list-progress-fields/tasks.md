## Specification Tasks

- [x] Promote `specs/agent-exec/spec.md` delta — list 進捗フィールド MUST 化
- [x] Review delta scenarios for coverage (running / finished / state-missing の 3 ケース)

## Acceptance #1 Failure Follow-up

- [x] Resolve duplicate "Requirement: list の JSON ペイロード" in canonical spec (lines 73 and 287) — ensure delta targets the correct instance
- [x] Fix delta to preserve `root`, `truncated`, `skipped` envelope fields from canonical requirement text
- [x] Fix delta to preserve `short_job_id` in required job entry fields
- [x] Remove `started_at` → `created_at` rename from delta (out of scope) or split into separate change

## Future Work

- 実装側の MUST 違反を検知する integration test 追加は `enrich-run-inline-completion` などと合わせて別途検討（本提案は canonical 化のみ）。
