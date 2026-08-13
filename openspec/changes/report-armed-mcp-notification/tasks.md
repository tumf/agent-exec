## Implementation Tasks

- [ ] Extend MCP `run` with optional launch-time completion sinks and validate them before creating or launching a workload. Completion condition: valid command/file sink input reaches canonical notification persistence; invalid or conflicting input returns a protocol-safe error with no workload process. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_persists_completion_sink_before_launch`)

- [ ] Add the optional structured `notification` response to successful MCP run results. Completion condition: a still-running job with persisted completion metadata returns `state="armed"`, generic sink classifications, `polling_required=false`, and the no-poll message; runs without a sink and failed admissions never claim armed state. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_reports_only_persisted_notification_as_armed`)

- [ ] Keep the MCP contract client-independent. Completion condition: core source and schemas contain no specific client, session, chat, or originating-client assumptions; multiple generic sink classes produce the same lifecycle semantics. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_notification_hint_is_client_independent`)

- [ ] Verify agents can stop observation based only on the response contract. Completion condition: MCP integration fixtures assert `polling_required=false` and the no-poll message for armed jobs, while unarmed jobs omit that claim and leave explicit observation available. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_armed_response_explains_next_action`)

- [ ] Apply and document the backward-compatible schema change. Completion condition: response types, checked-in JSON schema, schema command output, contract documentation, and version/changelog evidence agree on the optional notification field and minor schema version; existing fields retain their meanings. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test integration schema_command_matches_checked_in_schema`)

- [ ] Update agent guidance. Completion condition: documentation states that `notification.state="armed"` means the sink is persisted but delivery remains best effort, and agents must stop repeated observation unless explicitly asked for progress or diagnosing abnormal behavior; `state="running"` alone is not sufficient. (verification-id: mcp-armed-notification) (verification: integration - `python3 -c "from pathlib import Path; text=Path('skills/agent-exec/SKILL.md').read_text(); assert 'polling_required' in text and 'notification.state' in text and 'state=running' in text"`)

## Future Work

- Add optional host-specific adapters separately; they must translate their destination identity into generic MCP sink input without changing core response semantics.
- Add durable retry delivery separately if best-effort completion sinks prove insufficient.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate report-armed-mcp-notification --archive-gate`.
