# Changelog

Versioned history of the `schema_version` stdout JSON contract.

Backward-compatible additions (optional fields, new enum variants) bump MINOR.
Removals, type changes, meaning changes, and newly required fields bump MAJOR.

## schema 0.3

- Renamed the destructive launch control from `timeout` to `abandon-job-after`
  (`--abandon-job-after` on the CLI, `abandon_job_after` in MCP, HTTP `/exec`,
  and the public Rust request types). The old name did not say which lifetime it
  limited: `until` only bounds observation and never signals the job, while this
  control gives up on the job, terminates its process tree, and can permanently
  lose unfinished results. Every public description now starts with that
  warning.
- The control now requires an explicit acknowledgement at each public boundary:
  CLI `--acknowledge-result-loss`, MCP/HTTP `acknowledge_result_loss: true`, and
  the equivalent boolean on `run::RunOpts`, `create::CreateOpts`, and
  `embedded::RunRequest`. A nonzero limit without acknowledgement is rejected
  before the job is created.
- Removed the public `timeout` launch input on every surface. CLI `--timeout` is
  kept hidden purely as an always-failing migration trap, and MCP/HTTP `timeout`
  and `timeout_ms` return an actionable migration error before job creation.
  None of them can launch a job. The private `_supervise --timeout` process
  handoff keeps its wire spelling and is unchanged.
- Rust callers get compile-time breakage: `RunOpts::timeout_ms`,
  `SuperviseOpts::timeout_ms`, `CreateOpts::timeout_ms`, and
  `RunRequest::timeout_ms` are now `abandon_job_after_ms`, joined by the new
  `acknowledge_result_loss` boolean on the two launch request types.
- Added optional `abandoned_by` and `result_loss` fields to `state.json`,
  `status` (`StatusResponse`), `list` (`JobSummary`), and the `job.finished`
  completion event. They are written only when a configured limit actually
  terminated a workload; the terminal `state` value is unchanged, and a limit
  that never fired leaves both absent. Historical records without the fields
  stay readable and are never given synthesized provenance.
- Persisted job definitions dual-write equal `abandon_job_after_ms` and
  `timeout_ms` for one migration release, so an older binary that only reads
  `timeout_ms` keeps applying the same limit when it starts or restarts a job.
  Readers accept legacy-only, new-only, and equal dual definitions, and fail
  closed before start/restart when the two disagree, naming the job ID and both
  values. Dropping the legacy write requires a later explicit migration change.
- `--kill-after` is unchanged: it remains the delay between `SIGTERM` and
  `SIGKILL` after abandonment. The default runtime limit remains unlimited.
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
