## Implementation Tasks

- [x] Build a shared GC planner/executor for manual and automatic cleanup paths. Completion condition: `src/gc.rs` exposes internal functions or types that can evaluate job directories, classify state, compute bytes, apply retention/count/size rules, and execute deletion using one implementation for manual `gc` and auto-GC. (verification: unit - add `src/gc.rs` unit tests for duration parsing, terminal/non-terminal classification, cutoff comparison, count retention ordering, and byte budget candidate selection, then run `cargo test gc::`).

- [x] Extend manual `gc` CLI options and schema for cleanup controls and root summary. Completion condition: `agent-exec gc` accepts `--older-than`, `--max-jobs`, `--max-bytes`, and `--dry-run`; invalid duration/count/byte values fail deterministically; `GcData` includes summary fields without removing existing fields. (verification: integration - add `tests/integration.rs` cases that execute the compiled `agent-exec gc --dry-run` with mixed job directories and assert summary counts, candidate reasons, and no filesystem deletion).

- [x] Preserve GC deletion safety for all cleanup modes. Completion condition: both manual and automatic cleanup skip `running`, `created`, unreadable, and too-recent jobs; deletion is reported only after post-delete absence checks succeed; failures are counted and surfaced in manual GC output. (verification: integration - add `tests/integration.rs` cases proving `running` and `created` job directories remain after `agent-exec gc`, auto-GC from `agent-exec run`, and count/size cleanup pressure).

- [x] Add bounded auto-GC invocation to `run`. Completion condition: successful `agent-exec run` calls a best-effort auto-GC path after the new job is safely created/launched, honors default 30-day retention, and preserves all required `run` response fields and JSON-only stdout. (verification: integration - add a `tests/integration.rs` case that creates an old terminal job, executes `agent-exec run -- echo hi`, asserts the old job directory is removed, and validates the `run` JSON response still contains inline output fields).

- [x] Add bounded auto-GC invocation to `start`. Completion condition: successful `agent-exec start <job_id>` calls the same best-effort auto-GC path after launching a created job, without changing required `start` inline observation semantics. (verification: integration - add a `tests/integration.rs` case that creates an old terminal job plus a created job, runs `agent-exec start <created>`, asserts old terminal cleanup occurs, and verifies the started job remains inspectable via `agent-exec status`).

- [x] Add user controls for auto-GC opt-out and tuning. Completion condition: CLI/config-backed settings can disable auto-GC and configure retention/budget limits; unset settings preserve safe defaults; per-invocation opt-out prevents cleanup side effects. (verification: integration - add `tests/integration.rs` cases showing an auto-GC opt-out flag leaves an eligible old terminal job in place while normal `agent-exec run` or `agent-exec start` removes it).

- [x] Keep auto-GC best-effort and bounded. Completion condition: lock contention, unreadable entries, root absence, budget exhaustion, or individual deletion failures do not make the parent `run` / `start` command fail; manual `gc` continues to report failures explicitly. (verification: integration - add a `tests/integration.rs` fixture with at least one malformed or unreadable stale-looking job directory and assert `agent-exec run -- echo hi` succeeds while that directory is skipped rather than reported deleted).

- [x] Update public documentation for cleanup behavior. Completion condition: `README.md` documents automatic cleanup defaults, opt-out/tuning flags, manual `gc` count/size controls, and JSON summary fields. (verification: integration - add documentation assertions in `tests/integration.rs` if available, otherwise run `cargo test --test integration` after updating `README.md` and verify `README.md` contains auto-GC, `--max-jobs`, and `--max-bytes` cleanup guidance).

- [x] Run repository verification. Completion condition: formatting, linting, and tests pass with repository-standard commands. (verification: integration - run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Verification Follow-up
- Intermittent `tests/serve_integration.rs::test_auth_token_accepted` failure was observed once during an intermediate run, but targeted rerun and final full `cargo test --all` both passed.

## Future Work

- Consider a separate proposal for sharding the jobs root by date or cwd if flat root scans remain a bottleneck after bounded auto-GC.
- Consider a separate proposal for a dedicated `jobs summary` or `doctor` command if `gc --dry-run` summary is not enough for operator diagnostics.
