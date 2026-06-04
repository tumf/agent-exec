---
change_type: implementation
priority: high
dependencies:
  - add-rtk-compression-routing
references:
  - src/compress.rs
  - tests/integration.rs
  - rtk-ai/rtk/src/cmds/git/git.rs
---

# Add RTK-style Git Compression

**Change Type**: implementation

## Problem/Context

Git output is one of the highest-value RTK compression targets. The current `git` compression only filters a few generic lines and does not produce meaningful savings for `git log --stat`, `git status`, or large diffs.

## Proposed Solution

Add Git-specific compressors for observed outputs from `git status`, `git log`, `git diff`, `git show`, `git push`, `git pull`, `git branch`, and `git stash`.

## Acceptance Criteria

- `git log --stat` output is summarized by commit, preserving hash/subject and aggregating file stats.
- `git status` preserves branch, detached HEAD, and in-progress states such as rebase/merge/cherry-pick while removing hints.
- `git diff` and `git show` preserve changed file names, hunk headers, and per-file `+N -M` counts while bounding hunk bodies.
- `git push` and `git pull` strip progress noise and summarize successful outcomes in one line.
- Git compressors never replace raw observation fields and are subject to expansion guard.

## Explicit Completion Conditions

This change is complete when representative Git integration tests prove smaller compression for large outputs and correctness preservation for status state, diffs, push/pull summaries, and small-output guard cases.

## Dependencies

Requires `add-rtk-compression-routing`.

## Out of Scope

- Command rewriting or injecting RTK-specific git flags before execution.
- GitHub/GitLab CLI compression; that is covered by a separate proposal.
