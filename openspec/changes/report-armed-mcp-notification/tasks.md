## Implementation Tasks

- [ ] Extend MCP `run` with an optional launch-time completion command sink and validate it before creating or launching a workload. Completion condition: valid input reaches canonical `RunOpts.notify_command`; invalid or conflicting input returns a protocol-safe error with no workload process. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_persists_completion_sink_before_launch`)

- [ ] Add the optional structured `notification` response to successful MCP run results. Completion condition: a still-running job with persisted completion metadata returns `state="armed"`, `polling_required=false`, delivery classification, and the no-poll message; runs without a sink and failed admissions never claim armed state. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_reports_only_persisted_notification_as_armed`)

- [ ] Update the OpenCode auto-resume plugin to inject the validated callback through `tool.execute.before`. Completion condition: the normal path supplies the launch-time MCP notification input, removes post-response `notify set`, and preserves tool filtering, loopback restriction, session/job validation, shell quoting, and contained failures. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test opencode_integration plugin_injects_launch_time_callback_for_valid_agent_exec_runs`)

- [ ] Verify same-session continuation without observation polling. Completion condition: the deterministic OpenCode fixture receives an armed MCP response, performs no plugin-driven `wait`/`status`/`tail` calls, and the existing completion helper resumes the originating session once after terminal state. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test opencode_integration armed_run_resumes_originating_session_without_polling`)

- [ ] Apply and document the backward-compatible schema change. Completion condition: response types, checked-in JSON schema, schema command output, contract documentation, and version/changelog evidence agree on the optional notification field and minor schema version; existing fields retain their meanings. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test integration schema_command_matches_checked_in_schema`)

- [ ] Update agent and OpenCode integration guidance. Completion condition: documentation states that `notification.state="armed"` means the sink is persisted but delivery remains best effort, and agents must stop repeated observation unless explicitly asked for progress or diagnosing abnormal behavior. (verification-id: mcp-armed-notification) (verification: integration - `python3 -c "from pathlib import Path; text='\n'.join(Path(p).read_text() for p in ['skills/agent-exec/SKILL.md','examples/integrations/opencode-auto-resume/README.md']); assert 'polling_required' in text and 'armed' in text"`)

## Future Work

- Add equivalent pre-admission callback injection adapters for other agent clients only when each client can provide a trustworthy origin identity.
- Add durable retry delivery separately if best-effort completion sinks prove insufficient.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate report-armed-mcp-notification --archive-gate`.
