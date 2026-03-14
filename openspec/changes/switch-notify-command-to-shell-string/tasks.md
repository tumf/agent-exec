## Implementation Tasks

- [ ] Update CLI parsing and runtime types so `--notify-command` is treated as a shell command string in `src/main.rs`, `src/run.rs`, and `src/schema.rs` (verification: these files no longer parse/store a JSON argv array for command sinks)
- [ ] Implement platform-shell execution for command sinks while preserving stdin event delivery and `AGENT_EXEC_EVENT_*` env vars in `src/run.rs` (verification: command dispatch code launches via shell on Unix and Windows, and existing sink env behavior remains explicit in code)
- [ ] Update persisted metadata and completion delivery recording to store/report the command string target in `src/schema.rs` and any related serialization paths (verification: command sink metadata schema uses `String`, and delivery result target reflects the configured shell command)
- [ ] Revise integration coverage in `tests/integration.rs` for successful command-sink delivery, shell-based failure cases, and unchanged job terminal state after sink failure (verification: integration tests exercise `--notify-command` with a command string and pass on supported platforms)
- [ ] Update `README.md` to describe `--notify-command` as a shell command string, replace JSON-argv examples, and simplify the OpenClaw ad-hoc guidance (verification: `README.md` no longer documents `JSON_ARGV`, and command examples show shell-string usage consistently)
- [ ] Update `skills/agent-exec/SKILL.md` and `skills/agent-exec/references/completion-events.md` to match the new shell-string contract and operational guidance (verification: files under `skills/agent-exec/` no longer instruct users to pass JSON argv arrays and instead describe shell-based command sink behavior consistently)
- [ ] Run validation and repo checks for the proposal and implementation contract (verification: `python3 "/Users/tumf/.agents/skills/cflx-proposal/scripts/cflx.py" validate switch-notify-command-to-shell-string --strict` succeeds; implementation follow-up should run `cargo test --all`)

## Future Work

- Consider adding a separate explicit shell-free sink flag in the future if callers need literal argv execution without shell parsing.
