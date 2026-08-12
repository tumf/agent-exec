## Implementation Tasks

- [ ] Add the portable OpenCode global plugin under `examples/integrations/opencode-auto-resume/`. Completion condition: it observes the common `tool.execute.after` hook, filters recognized agent-exec MCP run tool names, extracts one validated job ID from successful output, validates loopback server and session identifiers, and calls `agent-exec notify set` without fixed user paths. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration plugin_attaches_callback_only_to_valid_agent_exec_runs`)

- [ ] Add the portable completion callback helper under `examples/integrations/opencode-auto-resume/`. Completion condition: it reads the persisted completion event from `AGENT_EXEC_EVENT_PATH`, defaults to a 60000 ms threshold, supports a documented threshold override, rejects invalid callback targets and identifiers, treats event/log content as untrusted, and invokes `opencode run --attach --session` only for eligible jobs. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration callback_applies_threshold_and_validates_delivery_target`)

- [ ] Make callback delivery idempotent per job. Completion condition: duplicate invocations for the same valid job create at most one OpenCode resume attempt, while a failed resume does not permanently suppress a later retry. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration callback_is_idempotent_and_retryable_after_failure`)

- [ ] Add deterministic integration fixtures and tests for the plugin/helper lifecycle. Completion condition: tests use temporary directories and fake `agent-exec`/`opencode` commands to cover the long-job success path, short-job no-op, exact threshold boundary, unrelated/failed tool calls, malformed outputs/events, non-loopback URLs, invalid IDs, duplicate delivery, and command failure without a live model, TUI, or external network. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration`)

- [ ] Document the reference integration and expose it from the project documentation. Completion condition: `examples/integrations/opencode-auto-resume/README.md` and the relevant README/docs link explain installation into `${OPENCODE_CONFIG_DIR:-$HOME/.config/opencode}/plugins`, helper placement, environment configuration, restart requirement, lifecycle, uninstall, best-effort delivery, loopback restriction, direct-CLI exclusion, and `role=user` event semantics without machine-specific paths. (verification-id: opencode-auto-resume-integration) (verification: integration - `python3 -c "from pathlib import Path; p=Path('examples/integrations/opencode-auto-resume/README.md').read_text(); assert 'OPENCODE_CONFIG_DIR' in p and 'role=user' in p and '60' in p"`)

- [ ] Run repository quality gates and verify no runtime/MCP contract changes. Completion condition: formatting, lint, all tests, and the focused integration test pass; the diff does not alter `src/mcp.rs`, the MCP run schema, or supervisor notification semantics. (verification-id: opencode-auto-resume-integration) (verification: integration - `prek run -a && cargo test --test opencode_integration && git diff --exit-code -- src/mcp.rs`)

## Future Work

- Add an installer command only after multiple integrations establish a stable installation contract.
- Add a durable retry worker only if best-effort completion callback delivery proves insufficient in real deployments.
- Consider other agent clients as separate reference integrations rather than adding client-specific behavior to the core runtime.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-opencode-auto-resume-integration --archive-gate`.
