## Implementation Tasks

- [x] Extend MCP `RunParams` and its generated tool schema with optional string fields `stdin` and `stdin_file`, preserving unknown-field rejection and documenting server-local path semantics (verification: integration - `tests/mcp_integration.rs` asserts `tools/list` exposes both fields; verification-id: mcp-stdin-tests)
- [x] Validate that `stdin` and `stdin_file` are mutually exclusive and that MCP never treats `stdin: "-"` as protocol-transport input; reject invalid calls before creating a job (verification: integration - `cargo test --test mcp_integration mcp_run_rejects_conflicting_stdin_without_creating_job`; verification-id: mcp-stdin-tests)
- [x] Wire MCP inline and file-backed values into the existing `StdinSource` and `run::run_response` path with `DEFAULT_STDIN_MAX_BYTES`, preserving canonical `stdin.bin`, metadata, supervisor handoff, and null stdin when omitted (verification: integration - `cargo test --test mcp_integration mcp_run_accepts`; verification-id: mcp-stdin-tests)
- [x] Add MCP integration coverage for exact inline bytes, server-local file snapshot behavior, omitted-input compatibility, missing/unreadable file rejection, and oversized input rejection using isolated roots and real managed commands (verification: integration - `cargo test --test mcp_integration`; verification-id: mcp-stdin-tests)
- [x] Document the new MCP `run` stdin surface in the `README.md` MCP tools reference, including mutual exclusion, server-local snapshot semantics, literal `"-"`, and the 64 MiB limit (verification: integration - `grep -n 'stdin_file' README.md` shows the updated `run` tool row and the following stdin paragraph; verification-id: mcp-stdin-tests)
- [x] Restore the four pre-existing `until`/empty-command scenarios in `openspec/changes/add-mcp-stdin-input/specs/agent-exec-mcp/spec.md` so archiving the MODIFIED requirement does not delete already-shipped contract text (verification: integration - `grep -c '^#### Scenario:' openspec/changes/add-mcp-stdin-input/specs/agent-exec-mcp/spec.md` reports 10, covering every scenario in `openspec/specs/agent-exec-mcp/spec.md`; verification-id: mcp-stdin-tests)
- [x] Run repository Rust quality gates and resolve only regressions introduced by this change (verification: integration - `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all`; verification-id: mcp-stdin-tests)

## Future Work

- Interactive or incremental stdin delivery would require a separate protocol and lifecycle design.
- Client-local file upload for remote MCP servers can be proposed separately if inline strings or server-local paths are insufficient.
- HTTP `serve` stdin support remains an independent API change.

## Notes

- evidence: `cargo test --test mcp_integration` -> 19 passed, 1 ignored (the pre-existing `heavy_mcp_wait_and_tail_preserve_running_job_semantics`).
- evidence: `cargo test --all` -> 138 + 2 + 270 + 19 + 24 passed, 0 failed, 2 pre-existing ignored.
- evidence: `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings` both clean.
- Implementation lives in `src/mcp.rs`: `RunParams` gains the two optional fields, the new `stdin_source` helper maps them onto `run::StdinSource`, and `run_response` receives `stdin` plus `run::DEFAULT_STDIN_MAX_BYTES`. No stdin storage or child piping was added outside `src/run.rs`.
- `stdin_source` is deliberately not `run::resolve_stdin_source`: that CLI helper maps `"-"` to `StdinSource::CallerStdin`, which MCP must never produce because process stdin is the JSON-RPC transport.
- `domain_error` now maps `run::StdinTooLarge` to the stable `stdin_too_large` code, matching the CLI boundary in `src/main.rs`. A missing or unreadable `stdin_file` has no dedicated CLI error type, so it stays `internal_error` per the proposal's "existing stable error mapping where available".
- `mcp_run_rejects_oversized_stdin_before_child_launch` is intentionally not `#[ignore]`d despite taking ~8s: CI runs `cargo test --all`, which skips ignored tests, so gating it would leave the required 64 MiB limit contract unverified.
- Pre-existing behavior left unchanged: when `stdin_file` names a missing/unreadable path, `materialize_stdin` in `src/run.rs:322` returns via `?` before its cleanup branch, leaving an empty `stdin.bin` in the job directory. The job records no `stdin_file` in `meta.json` and no supervisor starts, so the stray file is inert. This is shared CLI behavior and out of scope here; the integration test asserts the spec's actual requirement (no child launch) instead.

## Final Validation

Archive validation is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-mcp-stdin-input --archive-gate`
Result: `cflx openspec validate add-mcp-stdin-input --strict` and `--archive-gate` both pass.
