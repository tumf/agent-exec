## Implementation Tasks

- [ ] Add config discovery and parsing for `config.toml` under the XDG config path, plus `--config <PATH>` override plumbing in `src/main.rs` and the shared runtime option structs (verification: `src/main.rs` exposes the new flag and the runtime path resolution is covered by tests in `tests/integration.rs`).
- [ ] Introduce a shared shell-wrapper resolution layer that merges built-in defaults, config values, and `--shell-wrapper` CLI overrides for the active platform (verification: the launcher selection logic lives in one code path in `src/run.rs` or a helper module, and tests cover precedence).
- [ ] Update `run` command-string execution to use the resolved shell wrapper instead of a hardcoded platform launcher (verification: integration tests exercise a configured wrapper and observe the wrapped command execution path).
- [ ] Update `--notify-command` delivery to use the same resolved shell wrapper logic as `run` (verification: integration tests show a single configured wrapper affects notify-command delivery as well).
- [ ] Add config schema and persistence updates needed to represent effective shell-wrapper settings clearly in metadata or event records when the implementation chooses to persist them (verification: `src/schema.rs` and related assertions in `tests/integration.rs` match the documented contract).
- [ ] Update `README.md`, `skills/agent-exec/SKILL.md`, and related skill reference docs to describe config location, precedence, shared wrapper behavior, and examples (verification: these files mention `config.toml`, `--config`, `--shell-wrapper`, and the shared effect on `run` and `--notify-command`).
- [ ] Run `python3 "/Users/tumf/.agents/skills/cflx-proposal/scripts/cflx.py" validate make-shell-wrapper-configurable --strict` and repo verification commands such as `cargo test --all` after implementation (verification: proposal validates now; implementation follow-up records passing validation and tests).

## Future Work

- Consider whether wrapper metadata should be surfaced in additional status responses for debugging beyond the minimum needed by this proposal.
- Consider a future explicit shell-free execution mode if users need literal argv execution alongside the configurable shell-wrapper path.
