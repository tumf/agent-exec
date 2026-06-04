## Implementation Tasks

- [x] Add Git subcommand detection for `status`, `log`, `diff`, `show`, `push`, `pull`, `branch`, and `stash` (verification: unit - `src/compress.rs::tests::git_classifier_maps_representative_argv`, run by `cargo test git_`).
- [x] Implement `git log` block summarization that preserves hash/subject, keeps up to three meaningful body lines, removes common trailers, aggregates `N files changed`, `insertions`, and `deletions`, and caps commit count (verification: integration - `tests/integration.rs::compression_git_log_stat_is_smaller_and_preserves_commits`, run by `cargo test git_`).
- [x] Implement `git status` filtering that removes git hints while preserving branch/detached state and rebase/merge/cherry-pick/bisect/am state text when present (verification: unit - `src/compress.rs::tests::git_status_keeps_state_and_removes_hints` and `git_status_keeps_detached_and_rebase_state`, run by `cargo test git_`).
- [x] Implement `git diff` and `git show` summarization by changed file, hunk header, bounded hunk content, and per-file addition/deletion counts (verification: integration - `tests/integration.rs::compression_git_diff_summarizes_files_hunks_and_keeps_raw_stdout`, run by `cargo test git_`).
- [x] Implement `git push` and `git pull` progress-noise filtering with success and failure summaries (verification: unit - `src/compress.rs::tests::git_push_pull_branch_and_stash_summarize`, run by `cargo test git_`).
- [x] Implement `git branch` and `git stash` compact list/result summaries without hiding errors (verification: unit - `src/compress.rs::tests::git_push_pull_branch_and_stash_summarize`, run by `cargo test git_`).
- [x] Ensure all Git compressors go through shared expansion guard and leave raw fields untouched (verification: integration - `tests/integration.rs::compression_git_small_output_uses_expansion_guard_and_preserves_raw`, run by `cargo test git_`).
- [x] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- None for Git command compression; GitHub/GitLab CLI is handled separately.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-git-compression --archive-gate`
