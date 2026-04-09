## Implementation Tasks

- [ ] Update `src/main.rs` help text for `wait` so `--until` is the primary wait-deadline flag and the distinction from process runtime `--timeout` is explicit (verification: manual - `agent-exec wait --help` and `agent-exec run --help` show non-overlapping terminology).
- [ ] Replace legacy `--timeout-ms` examples with `--until` in `README.md` and any user-facing docs while preserving the current default 30,000ms semantics (verification: manual - README wait usage and examples use `--until` as the canonical spelling).
- [ ] Update `tests/integration.rs` so normative `wait` coverage uses `--until`, and keep at most one explicit backward-compatibility assertion for legacy `--timeout-ms` if the alias remains supported (verification: integration - wait deadline tests pass with `--until`, plus optional compatibility coverage for `--timeout-ms`).
- [ ] If `--timeout-ms` remains accepted, document it as deprecated / legacy in the proposal-aligned docs and ensure validation artifacts reflect `--until` as the canonical contract (verification: strict/manual - wording in docs and proposal stays aligned with `openspec/specs/agent-exec-run/spec.md`).

## Future Work

- Remove the legacy `--timeout-ms` alias entirely in a future breaking-release proposal once downstream callers have migrated.
