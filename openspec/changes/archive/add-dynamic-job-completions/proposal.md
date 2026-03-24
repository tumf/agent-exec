# Change Proposal: add-dynamic-job-completions

## Problem / Context

After `add-shell-completions` (Phase 1), subcommands, flags, and constrained
option values can be tab-completed. However, the most tedious argument to type
is `<job_id>` — a 26-character ULID used by `status`, `tail`, `wait`, `kill`,
`start`, `delete`, `tag set`, and `notify set`. Without dynamic completion,
users must copy-paste or rely on `agent-exec list` output.

Combined with `add-prefix-job-lookup` (active proposal), dynamic completion
makes the CLI dramatically easier to use: the user types a few characters,
presses Tab, and gets the full (or narrowed) job ID.

## Proposed Solution

Implement **dynamic shell completion for `<job_id>` arguments** using
`clap_complete`'s native completion infrastructure. The approach uses a
custom completer function registered via `value_parser` on each `<job_id>`
argument.

### Mechanism

1. **Custom completer**: a function/struct that implements the
   `clap_complete::engine::ValueCandidates` trait (or equivalent API for
   clap_complete v4). When invoked during completion:
   - Resolve the jobs root directory (same `resolve_root` logic in
     `src/jobstore.rs`).
   - List subdirectory names under the root.
   - Optionally read `state.json` to provide description annotations
     (e.g. `01HXYZ... — running`).
2. **Context-aware filtering** (best-effort, not required for MVP):
   - `start`: only `created` state jobs.
   - `kill`: only `running` state jobs.
   - `delete`: only terminal state jobs (`exited`, `killed`, `failed`).
   - `status`, `tail`, `wait`, `tag set`, `notify set`: all jobs.
3. **`--root` awareness**: if the user has already typed `--root <path>` before
   the job_id argument, the completer should use that path instead of the
   default root.

### Key design points

- Completion is best-effort: if the root directory is unreadable or
  `state.json` is malformed, the completer returns an empty list rather than
  erroring.
- Completion performance: directory listing is O(n) in the number of jobs.
  For typical agent workloads (<1000 jobs), this is fast enough. No caching
  is introduced in this proposal.
- The completer reuses `resolve_root()` from `src/jobstore.rs` — no
  duplication of root resolution logic.

## Acceptance Criteria

- [ ] Tab-completing `agent-exec status <TAB>` in a Bash/Zsh/Fish shell with
      the generated completion script installed lists existing job IDs.
- [ ] Tab-completing `agent-exec kill <TAB>` shows only running jobs (when
      context-aware filtering is implemented).
- [ ] Tab-completing with `--root /custom/path status <TAB>` lists jobs from
      the specified root.
- [ ] If the root directory does not exist, completion returns an empty list
      (no error).
- [ ] Integration test: the completion engine returns job IDs when queried
      programmatically (if `clap_complete` exposes a testable API).
- [ ] `prek run -a` passes (fmt, clippy, tests).

## Out of Scope

- Completion for tag names or tag filter patterns.
- Performance optimization (caching, indexing) for very large job stores.
- Nushell or Elvish support.
