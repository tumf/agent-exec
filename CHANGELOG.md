# Changelog

Versioned history of the `schema_version` stdout JSON contract.

Backward-compatible additions (optional fields, new enum variants) bump MINOR.
Removals, type changes, meaning changes, and newly required fields bump MAJOR.

## schema 0.2

- Added the optional `notification` object to `run` responses
  (`RunLikeResponse`). It is a client-independent completion-notification
  status carrying `state` (`"armed"`), `sinks` (`"command"` / `"file"`),
  `polling_required`, and an agent-readable `message`.
- `notification` is present only when a completion sink was persisted before the
  managed workload launched and the job is still non-terminal. `armed` records
  that terminal dispatch metadata exists; it does not guarantee downstream
  delivery. Responses omit the object entirely when nothing is asserted.
- MCP `run` gained the optional `notify_command` and `notify_file` inputs that
  persist such a sink through the canonical run notification path.
- No existing field was removed, retyped, or changed in meaning. Clients that
  ignore unknown optional fields remain compatible.

## schema 0.1

- Initial published contract: the `schema_version` / `ok` / `type` envelope and
  the `run`, `restart`, `status`, `tail`, `wait`, `kill`, `list`, and `schema`
  response shapes.
