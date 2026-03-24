## Implementation Tasks

- [x] Task 1: Research and select the `clap_complete` API for custom dynamic completions — confirm whether `CompleteEnv` + `ValueCandidates` is stable in clap_complete v4, or whether an alternative approach is needed (verification: document chosen API in code comments; `cargo check` passes)
- [x] Task 2: Create `src/completions.rs` module with a `JobIdCompleter` struct implementing the chosen completer trait (verification: `cargo check` passes)
- [x] Task 3: Implement `JobIdCompleter` core logic: resolve root via `resolve_root()`, call `std::fs::read_dir`, return directory names as candidates (verification: unit test with temp dir containing mock job dirs returns correct candidates)
- [x] Task 4: Add optional description annotation to candidates by reading `state.json` from each job dir to extract state (verification: unit test confirms description includes state string)
- [x] Task 5: Implement context-aware filtering: `start` returns `created` jobs, `kill` returns `running` jobs, `delete` returns terminal-state jobs, others return all jobs (verification: unit tests per subcommand context)
- [x] Task 6: Wire `JobIdCompleter` into `<job_id>` arguments across all subcommands: `status`, `tail`, `wait`, `kill`, `start`, `delete`, `tag set`, `notify set` via `value_parser` or `add(CompleteEnv)` (verification: `cargo check` passes; generated completion scripts reference the custom completer)
- [x] Task 7: Handle `--root` override in completer: if the partial command line includes `--root <path>`, use that path for root resolution (verification: unit test with explicit root shows jobs from that directory)
- [x] Task 8: Add integration tests verifying dynamic completion behavior where possible — at minimum, test that the completer function returns expected job IDs given a known root directory (verification: `cargo test --test integration` passes)
- [x] Task 9: Update generated completion scripts to support the dynamic completer (ensure `completions bash/zsh/fish/powershell` output includes hooks for dynamic completion) (verification: diff generated scripts before/after; dynamic hooks present)
- [x] Task 10: Run `prek run -a` and fix any fmt/clippy/test failures (verification: `prek run -a` exits 0)

## Future Work

- Completion for `--tag` values (enumerate existing tags from job metadata)
- Performance optimization: caching or indexing for large job stores (>1000 jobs)
- Nushell / Elvish dynamic completion support

## Acceptance #1 Failure Follow-up

- [x] Fix `--root` dynamic completion resolution for completion modes that do not provide `COMP_LINE` (e.g. parse completion argv/env fallback) so candidates come from the explicit root path.
- [x] Add an integration test that invokes dynamic completion with `--root <custom-path>` and verifies returned job-ID candidates come from that path.
