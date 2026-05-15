## Implementation Tasks

- [ ] Add `command` to the public list job summary schema. Completion condition: `src/schema.rs::JobSummary` contains `pub command: Vec<String>` and serde emits it as a normal required field in `jobs[]`. (verification: integration - a `tests/integration.rs` list test observes `jobs[].command` in CLI JSON output)
- [ ] Populate `command` from persisted job metadata in `list`. Completion condition: `src/list.rs` copies `meta.command` into each `JobSummary` without changing cwd, state, tag, sort, or limit filtering. (verification: integration - `tests/integration.rs::list_jobs_include_command` creates a multi-argument job and asserts `agent-exec list --all` returns the exact array in the matching job entry)
- [ ] Add regression coverage for command visibility. Completion condition: `tests/integration.rs` includes a test that launches or creates a job, runs `list`, finds the job by `job_id`, and asserts `command` equals the original argv in order. (verification: integration - `cargo test --test integration list_jobs_include_command`)
- [ ] Preserve repository CLI contracts. Completion condition: the implementation keeps stdout JSON-only and does not alter existing `list` filters, truncation, or ordering tests. (verification: integration - `cargo test --test integration list`)
- [ ] Run repository verification. Completion condition: formatting, linting, and tests pass with repository-standard commands. (verification: integration - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`)

## Future Work

- Consider a separate proposal for optional redacted or human-formatted command display if users need a non-JSON presentation layer.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-list-command-field --archive-gate`
