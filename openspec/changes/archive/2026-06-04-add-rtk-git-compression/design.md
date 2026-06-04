# Design: RTK-style Git Compression

## Overview

Git compression should approximate original RTK's command-specific behavior while respecting `agent-exec`'s observation-only contract.

## Command Handling

- `git log`: parse observed commit blocks and stat lines. Since `agent-exec` cannot inject `--pretty`, support common observed formats including `--oneline`, default commit headers, and `--stat` summaries.
- `git status`: strip prose hints and preserve repository state information.
- `git diff` / `git show`: parse diff sections from `diff --git`, hunk headers, and changed lines.
- `git push` / `git pull`: remove progress lines and summarize final refs or changed-file shortstat.

## Output Examples

`git log --stat -30` should become similar to:

```text
aed01dd chore: remove serena indexing setup
  2 files, +1 -18
3e4cca3 chore: Release agent-exec version 0.2.9
  2 files, +2 -2
... +26 commits omitted
```

`git diff` should become similar to:

```text
src/compress/git.rs
  @@ fn summarize_git_log(...) @@
  + added parser branch
  +42 -3
```

## Recovery

The full raw output remains recoverable from `stdout_log_path` / `stderr_log_path`; compression output should mention truncation via `strategy` rather than embedding raw recovery commands.
