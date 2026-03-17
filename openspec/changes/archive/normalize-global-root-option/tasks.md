## Implementation Tasks

- [x] Add a top-level `--root <PATH>` clap option in `src/main.rs` and thread it through command dispatch so all job-store subcommands consume a shared parsed value (verification: `src/main.rs` defines `root` once on `Cli`, and dispatch for `run`, `status`, `tail`, `wait`, `kill`, `gc`, and `list` passes the shared value into existing option structs).
- [x] Decide and implement the compatibility story for legacy per-subcommand `--root` syntax, keeping behavior deterministic during the migration window (verification: `--root` is declared with `global = true` on the top-level `Cli` struct in `src/main.rs`; clap accepts the flag in both `agent-exec --root PATH CMD` and `agent-exec CMD --root PATH` positions identically; the internal `_supervise` command retains its own `--supervise-root` flag for internal use; both equivalent forms documented in README).
- [x] Update integration coverage in `tests/integration.rs` for normalized global-root invocations across representative commands and for the selected legacy-compatibility behavior (verification: tests invoke `agent-exec --root <PATH> ...` and assert the same job-store resolution semantics as before; 5 new tests added: `global_root_flag_run`, `global_root_flag_status`, `global_root_flag_list`, `global_root_flag_gc`, `global_root_flag_takes_precedence_over_env`).
- [x] Update user-facing docs in `README.md` and any bundled skill/docs that describe CLI usage so examples consistently use global `--root` syntax and explain unchanged precedence semantics (verification: the docs reference `agent-exec --root <PATH> <subcommand> ...` and still document the precedence order).
- [x] Run strict proposal validation and repo verification after implementation (verification: `python3 "$SKILL_ROOT/scripts/cflx.py" validate normalize-global-root-option --strict` succeeds, and repo checks such as `cargo test --all` cover the CLI migration; all 84 tests pass, clippy passes).

## Future Work

- Consider a follow-up cleanup that removes any temporary legacy alias once downstream callers have migrated.
- Consider whether other repeated cross-command flags should be promoted to top-level global options for the same CLI consistency reasons.

## Acceptance #1 Failure Follow-up

- [x] Decide and enforce the legacy per-subcommand `--root` policy: decided to support `agent-exec <subcommand> --root <PATH> ...` as backward-compatible syntax; `global = true` in clap accepts the flag in both positions identically with no additional code needed.
- [x] Add integration coverage in `tests/integration.rs` that pins the chosen legacy behavior for per-subcommand `--root` syntax (verification: 4 new tests added: `subcommand_root_flag_compat_run`, `subcommand_root_flag_compat_status`, `subcommand_root_flag_compat_list`, `subcommand_root_flag_compat_gc`).
- [x] Update `README.md` and this task list wording so the documented migration story matches the actual CLI behavior (verification: README now documents both `agent-exec --root PATH CMD` and `agent-exec CMD --root PATH` as equivalent, with preferred form noted).

## Acceptance #2 Failure Follow-up

- [x] Update `Implementation Tasks` item 2 to truthfully reflect the selected compatibility behavior (legacy per-subcommand `--root` remains supported and documented), instead of claiming a clean-break removal.
