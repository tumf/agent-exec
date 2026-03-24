## Implementation Tasks

- [ ] Add the `delete` CLI surface and response schema in `src/main.rs` and `src/schema.rs`, including `delete <job_id>`, `delete --all`, and `--dry-run` semantics (verification: `src/main.rs` exposes the new subcommand shape and `src/schema.rs` defines a `type="delete"` response with deleted/skipped job entries).
- [ ] Implement single-job deletion and cwd-scoped bulk deletion in a dedicated module such as `src/delete.rs`, deleting only allowed states and returning stable errors for invalid targets (verification: implementation resolves the jobs root, inspects `meta.json`/`state.json`, rejects `running`, and scopes `--all` by persisted `meta.json.cwd`).
- [ ] Wire command dispatch and shared helpers so deletion removes the full job directory atomically from the caller-visible job store without changing unrelated commands (verification: `src/main.rs` dispatches into the new module and existing command modules remain behaviorally unchanged).
- [ ] Add integration coverage in `tests/integration.rs` for single deletion, running-job rejection, cwd-scoped `delete --all`, dry-run preservation, and post-delete `job_not_found` behavior (verification: targeted integration tests fail without the feature and pass after it).
- [ ] Update `README.md` with `delete` usage, cwd-scoped bulk cleanup examples, and the distinction between `delete` and `gc` (verification: README includes copy-pasteable `delete <job_id>` and `delete --all` examples and explains that `gc` remains the age-based root-wide cleaner).

## Future Work

- Consider a separate proposal for root-wide bulk deletion, richer filters (`--state`, `--tag`), or an explicit `--cwd <PATH>` override if operators need broader cleanup scopes.
- Consider a separate proposal for a forced `kill-and-delete` workflow if interactive users frequently need to remove running jobs in one step.
