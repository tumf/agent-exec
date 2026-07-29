## Implementation Tasks

- [ ] Extend MCP `RunParams` and its generated tool schema with optional string fields `stdin` and `stdin_file`, preserving unknown-field rejection and documenting server-local path semantics (verification: integration - `tests/mcp_integration.rs` asserts `tools/list` exposes both fields; verification-id: mcp-stdin-tests)
- [ ] Validate that `stdin` and `stdin_file` are mutually exclusive and that MCP never treats `stdin: "-"` as protocol-transport input; reject invalid calls before creating a job (verification: integration - `cargo test --test mcp_integration mcp_run_rejects_conflicting_stdin_without_creating_job`; verification-id: mcp-stdin-tests)
- [ ] Wire MCP inline and file-backed values into the existing `StdinSource` and `run::run_response` path with `DEFAULT_STDIN_MAX_BYTES`, preserving canonical `stdin.bin`, metadata, supervisor handoff, and null stdin when omitted (verification: integration - `cargo test --test mcp_integration mcp_run_accepts`; verification-id: mcp-stdin-tests)
- [ ] Add MCP integration coverage for exact inline bytes, server-local file snapshot behavior, omitted-input compatibility, missing/unreadable file rejection, and oversized input rejection using isolated roots and real managed commands (verification: integration - `cargo test --test mcp_integration`; verification-id: mcp-stdin-tests)
- [ ] Run repository Rust quality gates and resolve only regressions introduced by this change (verification: integration - `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all`; verification-id: mcp-stdin-tests)

## Future Work

- Interactive or incremental stdin delivery would require a separate protocol and lifecycle design.
- Client-local file upload for remote MCP servers can be proposed separately if inline strings or server-local paths are insufficient.
- HTTP `serve` stdin support remains an independent API change.

## Final Validation

Archive validation is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-mcp-stdin-input --archive-gate`
