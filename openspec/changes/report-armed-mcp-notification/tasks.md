## Implementation Tasks

- [x] Extend MCP `run` with optional launch-time completion sinks and validate them before creating or launching a workload. Completion condition: valid command/file sink input reaches canonical notification persistence; invalid or conflicting input returns a protocol-safe error with no workload process. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_persists_completion_sink_before_launch`)

- [x] Add the optional structured `notification` response to successful MCP run results. Completion condition: a still-running job with persisted completion metadata returns `state="armed"`, generic sink classifications, `polling_required=false`, and the no-poll message; runs without a sink and failed admissions never claim armed state. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_reports_only_persisted_notification_as_armed`)

- [x] Keep the MCP contract client-independent. Completion condition: core source and schemas contain no specific client, session, chat, or originating-client assumptions; multiple generic sink classes produce the same lifecycle semantics. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_notification_hint_is_client_independent`)

- [x] Verify agents can stop observation based only on the response contract. Completion condition: MCP integration fixtures assert `polling_required=false` and the no-poll message for armed jobs, while unarmed jobs omit that claim and leave explicit observation available. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_armed_response_explains_next_action`)

- [x] Apply and document the backward-compatible schema change. Completion condition: response types, checked-in JSON schema, schema command output, contract documentation, and version/changelog evidence agree on the optional notification field and minor schema version; existing fields retain their meanings. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test integration schema_command_matches_checked_in_schema`)

## Future Work

- Add optional host-specific adapters separately; they must translate their destination identity into generic MCP sink input without changing core response semantics.
- Add durable retry delivery separately if best-effort completion sinks prove insufficient.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate report-armed-mcp-notification --archive-gate`.

- evidence: `cargo test --all` passed (163 unit + 272 integration + 24 mcp + 24 serve + 14 embedded + 4 opencode + 2 embedded-consumer + 1 doctest; 2 pre-existing heavy tests ignored)
- evidence: `prek run -a` passed (trailing-whitespace, end-of-file-fixer, check-toml, check-yaml, cargo fmt --check, cargo clippy -D warnings, cargo test --all)
- evidence: `cflx openspec validate report-armed-mcp-notification --strict` passed
