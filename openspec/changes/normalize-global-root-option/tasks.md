## Implementation Tasks

- [ ] Add a top-level `--root <PATH>` clap option in `src/main.rs` and thread it through command dispatch so all job-store subcommands consume a shared parsed value (verification: `src/main.rs` defines `root` once on `Cli`, and dispatch for `run`, `status`, `tail`, `wait`, `kill`, `gc`, and `list` passes the shared value into existing option structs).
- [ ] Decide and implement the compatibility story for legacy per-subcommand `--root` syntax, keeping behavior deterministic during the migration window (verification: `src/main.rs` and related parsing tests or integration tests show whether old syntax is still accepted or rejected with a stable usage error).
- [ ] Update integration coverage in `tests/integration.rs` for normalized global-root invocations across representative commands and for the selected legacy-compatibility behavior (verification: tests invoke `agent-exec --root <PATH> ...` and assert the same job-store resolution semantics as before).
- [ ] Update user-facing docs in `README.md` and any bundled skill/docs that describe CLI usage so examples consistently use global `--root` syntax and explain unchanged precedence semantics (verification: the docs reference `agent-exec --root <PATH> <subcommand> ...` and still document the precedence order).
- [ ] Run strict proposal validation and repo verification after implementation (verification: `python3 "$SKILL_ROOT/scripts/cflx.py" validate normalize-global-root-option --strict` succeeds, and repo checks such as `cargo test --all` cover the CLI migration).

## Future Work

- Consider a follow-up cleanup that removes any temporary legacy alias once downstream callers have migrated.
- Consider whether other repeated cross-command flags should be promoted to top-level global options for the same CLI consistency reasons.
