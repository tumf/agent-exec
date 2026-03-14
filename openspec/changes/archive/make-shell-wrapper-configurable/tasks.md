## Implementation Tasks

- [x] Add config discovery and parsing for `config.toml` under the XDG config path, plus `--config <PATH>` override plumbing in `src/main.rs` and the shared runtime option structs (verification: `src/main.rs` exposes the new flag and the runtime path resolution is covered by tests in `tests/integration.rs`).
- [x] Introduce a shared shell-wrapper resolution layer that merges built-in defaults, config values, and `--shell-wrapper` CLI overrides for the active platform (verification: the launcher selection logic lives in one code path in `src/config.rs`, and tests cover precedence).
- [x] Update `run` command-string execution to use the resolved shell wrapper instead of a hardcoded platform launcher (verification: integration tests exercise a configured wrapper and observe the wrapped command execution path).
- [x] Update `--notify-command` delivery to use the same resolved shell wrapper logic as `run` (verification: integration tests show a single configured wrapper affects notify-command delivery as well).
- [x] Add config schema and persistence updates needed to represent effective shell-wrapper settings clearly in metadata or event records when the implementation chooses to persist them (verification: `src/config.rs` defines the `AgentExecConfig` and `ShellConfig` structs; the shell wrapper is not persisted in job schema as the design does not require it).
- [x] Update `README.md`, `skills/agent-exec/SKILL.md`, and related skill reference docs to describe config location, precedence, shared wrapper behavior, and examples (verification: these files mention `config.toml`, `--config`, `--shell-wrapper`, and the shared effect on `run` and `--notify-command`).
- [x] Run `python3 "/Users/tumf/.agents/skills/cflx-proposal/scripts/cflx.py" validate make-shell-wrapper-configurable --strict` and repo verification commands such as `cargo test --all` after implementation (verification: proposal validates; `cargo test --all` passes 70 tests with 6 new shell-wrapper tests added).

## Future Work

- Consider whether wrapper metadata should be surfaced in additional status responses for debugging beyond the minimum needed by this proposal.
- Consider a future explicit shell-free execution mode if users need literal argv execution alongside the configurable shell-wrapper path.

## Acceptance #1 Failure Follow-up

- [x] Apply the resolved shell wrapper to `run` command-string execution (not only `--notify-command`) so `agent-exec run -- '<command-string>'` actually executes through the configured wrapper (verification: `supervise()` now uses `build_cmd_str()` + shell wrapper for all child execution; `shell_wrapper_applied_to_run_command_string` integration test passes).
- [x] Preserve shell-wrapper argv fidelity between `run` and `_supervise` (avoid join/split round-trips that break quoted or space-containing arguments) (verification: `execute()` now serializes the resolved wrapper as JSON via `--shell-wrapper-resolved`; `_supervise` deserializes it directly without re-parsing; `shell_wrapper_argv_fidelity_across_run_supervise` integration test passes).
