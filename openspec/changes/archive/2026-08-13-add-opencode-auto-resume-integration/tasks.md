## Implementation Tasks

- [x] Add the portable OpenCode global plugin under `examples/integrations/opencode-auto-resume/`. Completion condition: it observes the common `tool.execute.after` hook, filters recognized agent-exec MCP run tool names, extracts one validated job ID from successful output, validates loopback server and session identifiers, and calls `agent-exec notify set` without fixed user paths. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration plugin_attaches_callback_only_to_valid_agent_exec_runs`)

- [x] Add the portable completion callback helper under `examples/integrations/opencode-auto-resume/`. Completion condition: it reads the persisted completion event from `AGENT_EXEC_EVENT_PATH`, defaults to a 60000 ms threshold, supports a documented threshold override, rejects invalid callback targets and identifiers, treats event/log content as untrusted, and invokes `opencode run --attach --session` only for eligible jobs. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration callback_applies_threshold_and_validates_delivery_target`)

- [x] Make callback delivery idempotent per job. Completion condition: duplicate invocations for the same valid job create at most one OpenCode resume attempt, while a failed resume does not permanently suppress a later retry. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration callback_is_idempotent_and_retryable_after_failure`)

- [x] Add deterministic integration fixtures and tests for the plugin/helper lifecycle. Completion condition: tests use temporary directories and fake `agent-exec`/`opencode` commands to cover the long-job success path, short-job no-op, exact threshold boundary, unrelated/failed tool calls, malformed outputs/events, non-loopback URLs, invalid IDs, duplicate delivery, and command failure without a live model, TUI, or external network. (verification-id: opencode-auto-resume-integration) (verification: integration - `cargo test --test opencode_integration`)

- [x] Document the reference integration and expose it from the project documentation. Completion condition: `examples/integrations/opencode-auto-resume/README.md` and the relevant README/docs link explain installation into `${OPENCODE_CONFIG_DIR:-$HOME/.config/opencode}/plugins`, helper placement, environment configuration, restart requirement, lifecycle, uninstall, best-effort delivery, loopback restriction, direct-CLI exclusion, and `role=user` event semantics without machine-specific paths. (verification-id: opencode-auto-resume-integration) (verification: integration - `python3 -c "from pathlib import Path; p=Path('examples/integrations/opencode-auto-resume/README.md').read_text(); assert 'OPENCODE_CONFIG_DIR' in p and 'role=user' in p and '60' in p"`)

- [x] Run repository quality gates and verify no runtime/MCP contract changes. Completion condition: formatting, lint, all tests, and the focused integration test pass; the diff does not alter `src/mcp.rs`, the MCP run schema, or supervisor notification semantics. (verification-id: opencode-auto-resume-integration) (verification: integration - `prek run -a && cargo test --test opencode_integration && git diff --exit-code -- src/mcp.rs`)

## Notes

- Deliverables: `examples/integrations/opencode-auto-resume/{agent-exec-auto-resume.js,opencode-agent-exec-resume,README.md}`, the driver fixture `tests/fixtures/opencode/drive-plugin.mjs`, the suite `tests/opencode_integration.rs`, and an `## OpenCode Integration` section in `README.md` linking the example.
- `cargo test --test opencode_integration` passed: 4 tests, 0 failures. It covers the three named task verifications plus `plugin_and_helper_complete_the_real_agent_exec_notification_loop`, which drives the shipped `agent-exec` binary end to end (real `notify set`, real detached supervisor dispatch) against fake `opencode`.
- `prek run -a`: trailing-whitespace, end-of-file-fixer, check-toml, check-yaml, `cargo fmt --check`, and `cargo clippy --all-targets --all-features -- -D warnings` all passed. `cargo test --all` passed every suite except one `tests/serve_integration.rs` case.
- That serve failure is a pre-existing load flake in an untouched suite, not a regression: the change has a zero-line diff against `src/`, `schema/`, and `Cargo.toml` (`git diff --exit-code HEAD -- src/ schema/ Cargo.toml` exits 0), so the HTTP server and its tests are byte-identical to the base commit. A standalone `cargo test --test serve_integration` on this tree passed 24/24, and the flake presented as an empty HTTP response body on a different test in each occurrence. `tests/opencode_integration.rs` had already finished before that suite started, so it did not contend with it.
- `git diff --exit-code HEAD -- src/mcp.rs` exits 0: the MCP run schema, supervisor notification semantics, and release behavior are unchanged. `examples/` is deliberately left out of the `Cargo.toml` `include` list so published-package contents stay as they are.
- Implementation decision recorded in `design.md`: the callback helper is a Node script rather than a POSIX shell script. The completion event embeds the job's own argv and cwd, so eligibility needs a real JSON parser rather than `grep`, and building the OpenCode invocation as an argv array keeps event content away from a shell entirely. Node is already required to load the OpenCode plugin, so this adds no dependency.
- `tests/opencode_integration.rs` is `#![cfg(unix)]`. The fake executables are `sh` scripts and the integration targets a loopback OpenCode server on a POSIX host; the file compiles to nothing on the Windows CI leg.

## Future Work

- Add an installer command only after multiple integrations establish a stable installation contract.
- Add a durable retry worker only if best-effort completion callback delivery proves insufficient in real deployments.
- Consider other agent clients as separate reference integrations rather than adding client-specific behavior to the core runtime.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-opencode-auto-resume-integration --archive-gate`.

Documentation check for the reference integration README passed:
`python3 -c "from pathlib import Path; p=Path('examples/integrations/opencode-auto-resume/README.md').read_text(); assert 'OPENCODE_CONFIG_DIR' in p and 'role=user' in p and '60' in p"`.
`python3 scripts/validate-site.py` also passed.
