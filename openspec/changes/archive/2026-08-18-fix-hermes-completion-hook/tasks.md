## Implementation Tasks

- [x] Rewrite `skills/agent-exec/scripts/hermes-notify-hook` to invoke fail-closed `hermes send --quiet --to "$HERMES_NOTIFY_TARGET" "job_id=$AGENT_EXEC_JOB_ID event_path=$AGENT_EXEC_EVENT_PATH"`: require non-empty `HERMES_NOTIFY_TARGET`, remove `hermes notify` and all LLM provider/model handling (`HERMES_NOTIFY_PROVIDER`, `HERMES_NOTIFY_MODEL`), keep the `HERMES_BIN` override, and update the usage header to show the target assignment embedded in `notify_command` rather than managed-child `--env` routing (verification: integration - `bash -n skills/agent-exec/scripts/hermes-notify-hook` plus the `hermes_notify_hook`-prefixed tests in `tests/integration.rs`; verification-id: hermes-hook-tests)
- [x] Add deterministic tests in `tests/integration.rs`, all named with the `hermes_notify_hook` prefix, that execute the tracked helper script directly: (a) success path with a fake `hermes` executable (via `HERMES_BIN` or an isolated `PATH`) that records argv to a file, asserting the exact argument vector `send --quiet --to <target> <message>` where `<message>` is `job_id=<id> event_path=<path>` with no provider/model/LLM arguments; (b) missing `HERMES_NOTIFY_TARGET` exits non-zero without invoking the fake binary; (c) missing binary exits non-zero with `HERMES_BIN` unset, `PATH` set to a directory containing no `hermes`, and `HOME` set to an empty temporary directory so the helper's `$HOME/.hermes/...` fallback cannot resolve a real installation; no network, credentials, or LLM may be used (verification: integration - `cargo test --test integration hermes_notify_hook`; verification-id: hermes-hook-tests)
- [x] Update `skills/agent-exec/references/hermes.md` with an MCP `run` example whose `notify_command` embeds the request-scoped target assignment (e.g. `HERMES_NOTIFY_TARGET='slack:C0123:171234.0001' /absolute/path/hermes-notify-hook`), states that `notification.state="armed"` is persisted-sink evidence only, and keeps the single `wait --forever` watcher solely as the fallback when no sink is persisted; add a `hermes_notify_hook`-prefixed doc test in `tests/integration.rs` asserting no fenced code block in that file contains `hermes notify` (prose prohibiting the obsolete command is allowed) (verification: integration - `cargo test --test integration hermes_notify_hook`; verification-id: hermes-hook-tests)

## Future Work

- Durable retries and downstream delivery acknowledgements require a separate proposal.

## Final Validation

Archive validation is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate fix-hermes-completion-hook --archive-gate`
