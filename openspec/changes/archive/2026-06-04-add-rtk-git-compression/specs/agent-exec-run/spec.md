## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

`run`/`start`/`restart`/`tail` は inline/tail 観測レスポンスに built-in compression view を追加できなければならない（MUST）。Compression は `--compress <mode>` または alias `--rtk <mode>` で制御できなければならず、外部 `rtk` コマンドを呼び出してはならない（MUST NOT）。Supported modes は `off|route|errors|tests|logs|git|json|summary` であり、`auto` を supported mode として受け付けてはならない（MUST NOT）。

Git command outputs routed through `route` or explicit `git` compression must use Git-specific compact views when the observed command is a supported Git subcommand (MUST). Git compression must preserve the information needed to understand repository state, commit identity, changed files, diff context, and push/pull outcome while removing progress noise, repeated boilerplate, and excessive hunks (MUST). Git compression must not rewrite commands or replace canonical raw observation fields (MUST NOT).

#### Scenario: git log stat output is summarized by commit

**Given**: `agent-exec run --rtk route -- git log --stat -30` observes multi-commit Git log output
**When**: route compression classifies the command as `git-log`
**Then**: `compression.stdout` preserves commit hashes and subjects for retained commits
**And**: per-commit file/insertion/deletion stats are summarized compactly
**And**: the compressed output is smaller than the raw observed stdout when enough commits are present

#### Scenario: git status preserves repository state

**Given**: a `git status` output describes a dirty tree or an in-progress rebase/merge/cherry-pick state
**When**: Git status compression is applied
**Then**: branch or detached HEAD information is preserved
**And**: in-progress state information is preserved
**And**: git hint prose such as `use "git add"` is removed from the compressed view

#### Scenario: git diff preserves file and hunk context

**Given**: a `git diff` or `git show` output contains multiple changed files and hunks
**When**: Git diff compression is applied
**Then**: changed file names are preserved
**And**: hunk headers are preserved
**And**: per-file additions and deletions are summarized
**And**: excessive hunk body lines are bounded

#### Scenario: git push and pull remove progress noise

**Given**: `git push` or `git pull` output includes progress lines and a final outcome
**When**: Git transport compression is applied
**Then**: progress boilerplate such as object enumeration and compression lines is omitted
**And**: a successful outcome is summarized in one compact line
**And**: failure output still preserves the error-bearing lines
