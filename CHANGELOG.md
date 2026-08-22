# Changelog

Versioned history of the `schema_version` stdout JSON contract.

Backward-compatible additions (optional fields, new enum variants) bump MINOR.
Removals, type changes, meaning changes, and newly required fields bump MAJOR.

## schema 0.3

- A running job's abandonment deadline is no longer fixed at launch. Added
  `agent-exec abandon set <job_id> --in <seconds> --acknowledge-result-loss` and
  `agent-exec abandon clear <job_id>`, with matching MCP `set_abandonment` /
  `clear_abandonment` tools, HTTP `PUT /abandon/{job_id}` and
  `DELETE /abandon/{job_id}`, and `EmbeddedClient::set_abandonment` /
  `clear_abandonment`. `--in` / `abandon_in` is measured from durable acceptance
  of the update, not from job start. Every set requires result-loss
  acknowledgement because the resulting action is still destructive; clear never
  signals the job and requires none. HTTP CORS preflight now permits `PUT` and
  `DELETE`.
- Added the `abandon_set` and `abandon_clear` response types, carrying the
  durable `abandon_revision` the operation committed, its phase, the accepted
  duration and absolute deadline, and `abandon_configured_by`.
- Added optional `abandon_job_after_ms`, `abandon_deadline`,
  `abandon_remaining_ms`, `abandon_revision`, and `abandon_configured_by` to
  `status` (`StatusResponse`). They project the deadline the supervisor will
  actually act on rather than the one the job was defined with. All five are
  null when no supervisor-authored control record exists; duration, deadline,
  and remaining time are also null when the control is disabled, and remaining
  time is null for terminal jobs. `abandon_remaining_ms` is computed at response
  time, clamped to the inclusive range `[0, abandon_job_after_ms]`, and is
  advisory: firing is decided by the supervisor's locked control transition.
  `status` stays read-only and never creates a control record.
- Jobs now carry a revisioned job-local `abandon_control.json`, authored by the
  supervisor before it publishes `running`. Updater and supervisor serialize
  through advisory locking on a fixed-name `abandon.lock`, and the record is
  replaced atomically. An update that commits first is observed before the old
  deadline can signal; a trigger that commits first makes later updates fail
  with `invalid_state` instead of claiming it prevented the abandonment.
- A running job without a supervisor-authored control record — one launched by a
  release that predates this contract — rejects set and clear with a stable
  `invalid_state` error carrying restart guidance, and mutates nothing. Created,
  terminal, and already-triggered jobs are rejected the same way.
- The supervisor now runs its control observer for every managed workload,
  including unlimited jobs with progress reporting disabled, so a deadline set
  after launch is honoured. It re-reads the current locked revision before
  signaling and never fires because a timer merely expired.
- Restart re-arms a `launch`-configured control from its configured duration and
  preserves the accepted absolute deadline and revision of an `update`-configured
  one, so it never restores a launch-time deadline over a runtime change. A
  transition persisted as `triggered` whose workload is still alive is resumed
  rather than silently replaced.
- Every accepted set/clear synchronizes both dual-written compatibility duration
  fields; a clear writes zero to both, so a downgrade cannot resurrect a deadline
  the operator removed.
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
