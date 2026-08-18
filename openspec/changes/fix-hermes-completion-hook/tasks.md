## Implementation Tasks

- [ ] Replace the obsolete Hermes helper invocation with fail-closed `hermes send --quiet --to "$HERMES_NOTIFY_TARGET"`, preserving only canonical job/event evidence in the message (verification: integration - `bash -n skills/agent-exec/scripts/hermes-notify-hook` plus the helper test under `tests/`; verification-id: hermes-hook-tests)
- [ ] Add a deterministic repository-local helper test covering exact success argv, missing target, missing binary, and absence of LLM/provider/model arguments (verification: integration - `cargo test --test mcp_integration mcp_run_reports_armed_completion_sink` executes the tracked helper with a fake Hermes binary; verification-id: hermes-hook-tests)
- [ ] Update Hermes integration guidance with an MCP `run` example that embeds the request-scoped target assignment in `notify_command`, treats `armed` as persisted-sink evidence only, and uses one watcher only when no sink exists (verification: integration - `cargo test --test mcp_integration mcp_run_reports_armed_completion_sink` reads `skills/agent-exec/references/hermes.md` and rejects executable `hermes notify` guidance; verification-id: hermes-hook-tests)

## Future Work

- Durable retries and downstream delivery acknowledgements require a separate proposal.

## Final Validation

Archive validation is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate fix-hermes-completion-hook --archive-gate`
