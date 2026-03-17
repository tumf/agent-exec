# Change Proposal: normalize-global-root-option

## Problem/Context

`agent-exec` uses `--root` as the job-store selector across multiple commands, but the flag is currently defined separately on each subcommand.

- The current CLI shape repeats the same option and help text on `run`, `status`, `tail`, `wait`, `kill`, `gc`, and `list`.
- The repository already treats root resolution as a shared concern with a stable precedence order in `src/jobstore.rs` and `openspec/specs/agent-exec/spec.md`.
- Recent proposal work around a future `notify set` subcommand highlighted that `--root` behaves conceptually like a global option even though the clap definition is local to each command.
- Repeating `--root` per subcommand makes the syntax less uniform and increases the maintenance cost for help text, docs, and future subcommands.

## Proposed Solution

Promote `--root <PATH>` to a top-level CLI option and normalize command examples around the global form.

- Add `--root <PATH>` to the top-level `Cli` parser so it applies uniformly to all subcommands that operate on the job store.
- Thread the resolved root through command dispatch instead of requiring each subcommand to parse its own copy of the flag.
- Normalize documentation and examples to prefer `agent-exec --root <PATH> <subcommand> ...`.
- Preserve a compatibility path for existing users during migration, either by continuing to accept subcommand-local `--root` temporarily or by providing a clearly documented transition with equivalent behavior.
- Keep the existing root-resolution precedence unchanged once a root value is selected.

## Acceptance Criteria

- `agent-exec --root /tmp/jobs status <job_id>` and the equivalent normalized forms for `run`, `tail`, `wait`, `kill`, `list`, and `gc` are supported.
- Root resolution precedence remains `--root` -> `AGENT_EXEC_ROOT` -> `$XDG_DATA_HOME/agent-exec/jobs` -> platform default.
- The CLI help and `README.md` consistently present `--root` as a global job-store selector instead of repeating it as unrelated per-subcommand syntax.
- The implementation defines root parsing in one shared clap location and command dispatch passes that value uniformly into the existing command handlers.
- Integration coverage verifies normalized global syntax and whichever compatibility behavior is chosen for legacy per-subcommand `--root` usage.

## Out of Scope

- Changing the meaning of `cwd` or any command-specific execution directory behavior.
- Changing root-resolution precedence or storage layout on disk.
- Adding new notification-management subcommands in this proposal.
