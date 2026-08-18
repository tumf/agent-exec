# Changelog

Versioned history of the `schema_version` stdout JSON contract.

Backward-compatible additions (optional fields, new enum variants) bump MINOR.
Removals, type changes, meaning changes, and newly required fields bump MAJOR.

## schema 0.3

- Added optional execution-diagnostic fields to `status` responses
  (`StatusResponse`): `command`, `cwd`, `tags`, `pid`, `process_alive`,
  `updated_at`, `elapsed_ms`, `duration_ms`, `signal`, `logs_drained`,
  `stdout_log_path`, `stderr_log_path`, `stdout_total_bytes`, and
  `stderr_total_bytes`. All are derived from already-persisted job metadata,
  state, and log-file metadata.
- `state` keeps its persisted meaning. A stale `running` record is reported as
  `state="running"` with `process_alive=false`; `list` presents the same record
  as `unknown`. Neither read command rewrites `state.json`.
- `elapsed_ms` is live (response time minus `started_at`) and present only for
  non-terminal jobs that have started. `duration_ms` keeps its existing terminal
  wall-clock meaning and is emitted only when persisted.
- `process_alive` is a best-effort, same-user-scoped probe resolved only for
  persisted `running` state. Omission means no live observation was made; it is
  not an authoritative liveness guarantee and does not defend against PID reuse.
- Log byte totals come from bounded file-size metadata queries. `status` never
  reads log contents, and missing or unreadable logs report `0`.
- Corrected pre-existing `StatusResponse` schema drift: `created_at` is now
  defined and required, `started_at` is optional (absent in `created` state),
  and `created` is an allowed `state` value.
- `schema_version` is a global contract version, so completion (`job.finished`)
  and output-match (`job.output.matched`) event envelopes also carry
  `"0.3"`. Their field shapes are unchanged by this release.
- No existing field was removed, retyped, or changed in meaning. Clients that
  ignore unknown optional fields remain compatible.

## schema 0.2

- Added the optional `notification` object to `run` responses
  (`RunLikeResponse`). It is a client-independent completion-notification
  status carrying `state` (`"armed"`), `sinks` (`"command"` / `"file"`),
  `polling_required`, and an agent-readable `message`.
- `notification` is produced by MCP `run`, and only when a completion sink was
  persisted before the managed workload launched and the job is still
  non-terminal. CLI and HTTP responses omit it today. `armed` records
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
