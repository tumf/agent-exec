## Implementation Tasks

- [ ] Add CLI parsing and JSON response types for `agent-exec notify set <JOB_ID> --command <COMMAND>` in `src/main.rs` and `src/schema.rs` (verification: command dispatch includes `notify set`, and the new success envelope shape is defined in `src/schema.rs`).
- [ ] Implement metadata update logic that loads `meta.json`, replaces `notification.notify_command`, preserves `notification.notify_file`, and writes the result atomically via `src/jobstore.rs` helpers (verification: the implementation reads and writes `meta.json` without touching unrelated persisted fields).
- [ ] Change completion notification dispatch in `src/run.rs` so terminal delivery consults the latest persisted `meta.json.notification` before invoking sinks (verification: the completion-event path no longer depends solely on launch-time `RunOpts.notify_*` values for dispatch decisions).
- [ ] Add integration coverage in `tests/integration.rs` for successful metadata updates on existing jobs, preservation of `notify_file`, and use of an updated command sink before job completion (verification: tests fail without the new behavior and pass with it).
- [ ] Add integration coverage for terminal-job updates and missing-job errors, then update `README.md` with the new subcommand and its no-immediate-delivery semantics (verification: `tests/integration.rs` covers both cases and `README.md` documents `notify set`).

## Future Work

- Consider follow-up commands for inspecting, clearing, replaying, or immediately sending notifications once the basic metadata update workflow is in place.
