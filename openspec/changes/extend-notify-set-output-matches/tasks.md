## Implementation Tasks

- [x] Extend persisted notification schema and JSON response types in `src/schema.rs` so completion settings and output-match settings can coexist under `meta.json.notification` (verification: `src/schema.rs` defines fields for output pattern, match mode, stream, and output sinks without removing completion notification fields).
- [x] Update CLI parsing and command dispatch for `agent-exec notify set` in `src/main.rs` to accept output-match configuration while preserving the existing completion-oriented `--command` path from `add-notify-set-command` (verification: `src/main.rs` exposes `notify set` flags for output matching and keeps argument validation focused on metadata updates).
- [x] Implement metadata mutation helpers in `src/jobstore.rs` and the notify command handler so `notify set` writes output-match settings atomically, preserves unspecified notification fields, and never triggers immediate sink execution (verification: `meta.json` updates are atomic and integration tests show no command/file sink side effects during `notify set`).
- [x] Update supervisor delivery in `src/run.rs` to reload persisted notification metadata for future output lines, evaluate configured matches on stdout/stderr line boundaries, and emit `job.output.matched` events for every match while leaving `job.finished` delivery intact (verification: `src/run.rs` no longer depends only on launch-time notify values for output matching, and completion delivery still reads latest persisted metadata).
- [x] Persist output-match delivery attempts in a dedicated per-job record such as `notification_events.ndjson` and keep sink failures non-fatal to job state updates (verification: the implementation writes per-match delivery results and `state.json` / `completion_event.json` semantics remain unchanged for lifecycle outcomes).
- [x] Add integration coverage in `tests/integration.rs` for `notify set` output-match updates on running and terminal jobs, per-match command/file sink delivery, no replay of pre-existing output, regex and stream filtering behavior, and sink-failure non-destructiveness (verification: targeted tests fail without the new behavior and pass with it).
- [x] Update `README.md` and any related skill or usage docs to explain `notify set` output-match configuration, event semantics, per-match execution, and the no-immediate-delivery / no-replay constraints (verification: docs mention `job.output.matched`, `notify set` output flags, and every-match behavior).

## Future Work

- Support multiple concurrent output-match rules per job.
- Add commands for viewing, clearing, replaying, or manually sending persisted notification configurations.

## Acceptance #1 Failure Follow-up

- [x] Reload output-match notification config for each newly observed line (or otherwise guarantee `notify set` updates are visible before any subsequent line is evaluated) so immediate post-update matches are not dropped.
- [x] Add an integration test that configures `notify set --output-pattern` immediately before a near-future matching line (e.g., ~50ms) and verifies `job.output.matched` is delivered.
