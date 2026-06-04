## Implementation Tasks

- [ ] Add Git subcommand detection for `status`, `log`, `diff`, `show`, `push`, `pull`, `branch`, and `stash` (verification: unit - classifier tests map representative argv arrays to `git-*` detected kinds).
- [ ] Implement `git log` block summarization that preserves hash/subject, keeps up to three meaningful body lines, removes common trailers, aggregates `N files changed`, `insertions`, and `deletions`, and caps commit count (verification: integration - `agent-exec run --rtk route -- git log --stat -30` produces smaller `compression.stdout` with commit hashes and stat summaries).
- [ ] Implement `git status` filtering that removes git hints while preserving branch/detached state and rebase/merge/cherry-pick/bisect/am state text when present (verification: unit - fixtures for clean tree, dirty tree, detached HEAD, and rebase state).
- [ ] Implement `git diff` and `git show` summarization by changed file, hunk header, bounded hunk content, and per-file addition/deletion counts (verification: integration - fixture command output compresses to file/hunk summaries and remains smaller than raw).
- [ ] Implement `git push` and `git pull` progress-noise filtering with success and failure summaries (verification: unit - push progress fixture returns `ok <ref>` or `ok (up-to-date)`; pull fixture returns `ok N files +X -Y`).
- [ ] Implement `git branch` and `git stash` compact list/result summaries without hiding errors (verification: unit - branch list and stash list fixtures are compacted with counts and current item preserved).
- [ ] Ensure all Git compressors go through shared expansion guard and leave raw fields untouched (verification: integration - small Git output triggers guard or omits oversized candidate while canonical stdout remains raw).
- [ ] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- None for Git command compression; GitHub/GitLab CLI is handled separately.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-git-compression --archive-gate`
