# Design

## Decision

Notification registration becomes part of MCP `run` admission rather than a post-response side effect.

The MCP client may supply a completion command sink. The server validates and persists it through the existing `RunOpts.notify_command` path before supervisor launch. Only a successful response for a still-running job with that persisted sink may report notification as armed.

## Response Shape

```json
{
  "notification": {
    "state": "armed",
    "delivery": "originating_session",
    "polling_required": false,
    "message": "Completion notification is armed. Do not poll wait/status/tail; this session will resume when the job finishes."
  }
}
```

`notification` is optional. Absence means the MCP response does not assert any completion delivery. Callers must not infer armed state from plugin installation alone.

## OpenCode Adapter

The plugin moves callback construction to `tool.execute.before`:

1. Validate the agent-exec MCP run tool name.
2. Validate loopback OpenCode server URL and originating session ID.
3. Add the completion command sink to the run arguments.
4. Let MCP admission persist the sink and produce the truthful response hint.
5. At terminal state, the existing helper reads the persisted event and resumes the same session.

The adapter remains OpenCode-specific. Core MCP fields describe a generic command sink plus a generic machine-readable notification status; core code does not know OpenCode session semantics except for a caller-supplied delivery label/message if the chosen schema permits those values.

## Trust and Failure Boundaries

- Completion command input is server-local privileged configuration, equivalent to existing CLI `--notify-command`; document that MCP server access grants this capability.
- The plugin only targets loopback OpenCode servers and validates identifiers before constructing shell-quoted argv.
- Invalid sink input fails before workload launch.
- Sink execution remains best effort and does not alter job outcome.
- `armed` means the sink was persisted for terminal dispatch, not guaranteed downstream delivery.
- Job output and completion-event content remain untrusted and are not interpolated into the continuation prompt.

## Compatibility

The new response object is an optional field. Under the repository's versioning policy, this requires a minor schema bump and a corresponding schema changelog entry. Existing clients may ignore the unknown field. Existing MCP calls without the new input retain current behavior.

## Rejected Alternatives

### Post-response result mutation

OpenCode does not reliably propagate `tool.execute.after` mutations to the content seen by the model. It also cannot make registration atomic with admission.

### Skill-only guidance

A skill can discourage polling but cannot prove callback registration succeeded. It leaves correctness dependent on prompt adherence.

### Always return a generic hint

Returning “notification armed” without a persisted sink is false. The response must reflect actual admitted job metadata.
