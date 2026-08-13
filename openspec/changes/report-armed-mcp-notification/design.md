# Design

## Decision

Notification registration becomes part of client-independent MCP `run` admission.

Any MCP client may supply a completion sink. The server validates and persists it through the existing canonical notification path before supervisor launch. Only a successful response for a still-running job with that persisted sink may report notification as armed.

## Response Shape

```json
{
  "notification": {
    "state": "armed",
    "sinks": ["command"],
    "polling_required": false,
    "message": "Completion notification is armed through the configured sink. Do not poll wait/status/tail."
  }
}
```

`notification` is optional. Absence means the MCP response does not assert any completion delivery. Callers must not infer armed state from host configuration alone.

The fields describe only generic job lifecycle state:

- `state="armed"`: terminal dispatch metadata was persisted before launch.
- `sinks`: the generic configured sink classes, such as `command` and `file`; both may be present.
- `polling_required=false`: normal completion observation should be delegated to that sink.
- `message`: a concise agent-readable rendering of the same machine state.

No field names a specific client, session, chat, or another host-specific destination.

## Client Adapters

A host-specific adapter may construct a sink before calling MCP `run`. For example, an adapter can route a completion event to a session, webhook, local event file, or queue. That adapter owns destination identity and delivery semantics. Core MCP only validates/persists the supplied sink and reports whether it was armed.

Adapters are optional. A raw MCP client can provide the sink directly. A client that provides no sink receives no armed claim and may use explicit observation commands.

## Trust and Failure Boundaries

- Completion command input is server-local privileged configuration, equivalent to existing CLI `--notify-command`; document that MCP server access grants this capability.
- Invalid sink input fails before workload launch.
- Sink execution remains best effort and does not alter job outcome.
- `armed` means the sink was persisted for terminal dispatch, not guaranteed downstream delivery.
- Job output and completion-event content remain untrusted.
- Host-specific credentials, destination IDs, and continuation behavior stay outside generic core logic.

## Compatibility

The new response object is an optional field. Under the repository's versioning policy, this requires a minor schema bump and a corresponding schema changelog entry. Existing clients may ignore the unknown field. Existing MCP calls without the new input retain current behavior.

## Rejected Alternatives

### Client-specific response fields

A destination-specific field would incorrectly assume a session-oriented host. Generic core cannot know what the sink means downstream.

### Post-response result mutation

A client plugin can attach a sink after `run`, but MCP cannot truthfully report it as armed and registration races with fast completion.

### Skill-only guidance

A skill can discourage polling but cannot prove sink registration succeeded. It leaves correctness dependent on prompt adherence.

### Always return a generic hint

Returning “notification armed” without a persisted sink is false. The response must reflect actual admitted job metadata.
