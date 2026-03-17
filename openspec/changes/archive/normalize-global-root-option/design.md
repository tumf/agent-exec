# Design: normalize-global-root-option

## Summary

This change makes `--root` a true global CLI option while preserving the existing job-store semantics. The design goal is to reduce repeated clap definitions and make command syntax consistent without changing how roots are resolved or how job data is stored.

## Current State

- `src/main.rs` defines `root: Option<String>` separately on multiple subcommands.
- Each command handler passes its own local `root` field into existing option structs.
- `src/jobstore.rs` already centralizes the root-resolution rules, so the duplication is in CLI parsing rather than storage behavior.

## Proposed CLI Shape

Preferred syntax after this change:

```bash
agent-exec --root /tmp/jobs run -- echo hi
agent-exec --root /tmp/jobs status <job_id>
agent-exec --root /tmp/jobs list --all
```

## Compatibility Strategy

To avoid an unnecessary breaking change, this proposal assumes a migration window.

- The CLI should prefer the top-level `--root` in help text and examples.
- Legacy per-subcommand `--root` may remain temporarily as a compatibility alias if clap wiring allows it cleanly.
- If both forms are accepted during migration, the precedence must be explicit and deterministic; the recommended rule is to reject ambiguous double-specification with a usage error instead of silently choosing one.

This proposal does not require the compatibility alias to remain forever. It only requires the migration behavior to be documented and tested.

## Dispatch Model

The top-level `Cli` struct should own `root: Option<String>`, and the match arms in `run(cli)` should forward that shared value into the existing command option structs.

This keeps:

- root parsing in one place,
- root semantics unchanged in `resolve_root`, and
- command implementation modules (`src/run.rs`, `src/status.rs`, `src/tail.rs`, `src/wait.rs`, `src/kill.rs`, `src/gc.rs`, `src/list.rs`) focused on behavior rather than CLI duplication.

## Verification Impact

Integration coverage should prove two things:

1. The new global syntax resolves the same job store as the old syntax.
2. The chosen migration behavior for legacy per-subcommand `--root` is stable and documented.
