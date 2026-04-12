# Design: remove-wait-timeout-ms-alias

## Summary
`wait --timeout-ms` is currently accepted by clap even though canonical OpenSpec has already replaced it with `--until`. This change removes the alias entirely and aligns tests and distributed skill documentation with the canonical contract.

## Premise / Context
- `src/main.rs` currently defines `wait.until` with `aliases = ["timeout-ms"]`, so the CLI silently accepts the removed spelling.
- `tests/integration.rs` still contains a compatibility assertion for `wait --timeout-ms`, which means the test suite would currently fail once the implementation is corrected unless the tests are updated together.
- `README.md` and `skills/agent-exec/SKILL.md` still advertise the legacy alias, so users can be taught behavior that the canonical spec forbids.
- The repo's `AGENTS.md` explicitly requires CLI-contract changes to update tests and validation commands, so the proposal must carry verification ownership across implementation and docs.

## Design Decisions

### 1. Treat this as a contract bug, not a deprecation phase
Canonical spec already says `--timeout-ms` was replaced by `--until`, not that it remains optional compatibility. Therefore the implementation change should remove the alias rather than keep a warning-only path.

### 2. Make rejection observable through integration tests
Because clap usage errors bypass JSON output, the regression test must assert the expected command-line contract directly: exit code 2, empty stdout, and stderr usage text. This protects against accidental reintroduction of the alias.

### 3. Align distributed guidance at the same time
The repo-local skill file is part of the shipped operational guidance for `agent-exec`. Leaving old wording there would recreate the same confusion even after the binary is fixed. The docs and skill update therefore belong in the same proposal scope.

## Verification Strategy
- Integration coverage for valid `wait`, `wait --until`, `wait --forever`, and invalid `wait --timeout-ms`.
- Search-based/manual verification that `README.md` and `skills/agent-exec/SKILL.md` no longer teach `--timeout-ms` for wait deadlines.
- Full repo verification with fmt, clippy, and tests because this is a CLI contract change.
