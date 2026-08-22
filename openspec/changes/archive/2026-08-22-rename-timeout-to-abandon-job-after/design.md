# Design: distinguish observation from workload abandonment

## Naming boundary

- `until`: caller observation duration. Expiry returns control and never signals the job.
- `abandon-job-after` / `abandon_job_after`: an explicit decision to give up on the job. Expiry sends `SIGTERM`, may escalate to `SIGKILL`, and can permanently lose unfinished results.

`runtime-timeout`, `max-runtime`, and `terminate-job-after` were rejected. They sound like ordinary scheduling controls and understate result loss. `abandon-job-after` states that the caller is giving up on completion rather than merely ending observation.

## Warning and acknowledgement

Naming is the first warning, not the only warning. CLI help and structured API field descriptions begin with: `WARNING: Gives up on the job, terminates it, and may permanently lose unfinished results. Use until to stop waiting without stopping the job.`

The destructive control requires a separate acknowledgement at each public boundary: CLI `--acknowledge-result-loss`, MCP/HTTP `acknowledge_result_loss=true`, and an equivalent explicit public Rust request boolean. Missing acknowledgement fails before persistence or launch.

The acknowledgement is launch admission friction, not the durable proof of what happened. It may be supplied mechanically. Durable state therefore records actual abandonment separately through `abandoned_by="abandon_job_after"` and `result_loss=true` when the deadline causes termination. A configured deadline that never fires does not set those markers.

The private `_supervise` handoff is not a public admission boundary. Its private `--timeout` wire spelling remains unchanged and receives an already admitted/persisted duration from the parent. Start/restart of an existing definition does not request acknowledgement again; admission occurred when the definition was created, or predates this contract for a legacy job.

## Public legacy input rejection

No legacy public input remains capable of launch.

- CLI keeps `--timeout` hidden only as an always-failing migration trap. The error names `--abandon-job-after ... --acknowledge-result-loss` and `--until`.
- MCP and HTTP parse legacy `timeout` / `timeout_ms` into an explicit migration error before creating a job instead of returning an unactionable generic unknown-field error.
- Public Rust request structs expose only the new field. Rust callers receive compile-time breakage and migration guidance in release notes.

This is not a deprecated alias: the old input can never execute a workload.

## Persisted metadata compatibility

Old definitions already exist and an older binary may operate a definition created during the migration release. Read compatibility alone is insufficient: if a new writer emits only `abandon_job_after_ms`, an older binary defaults missing `timeout_ms` to zero and silently removes the limit.

The migration release therefore uses this contract:

1. Writers persist equal `abandon_job_after_ms` and `timeout_ms` values, including zero.
2. Readers accept legacy-only, new-only, and equal dual-field definitions.
3. Readers reject unequal dual fields before start/restart with an error containing the job ID and both field names/values.
4. Internal code resolves both forms into one canonical `abandon_job_after_ms` value.
5. A later explicit migration change may stop writing `timeout_ms` after the downgrade window. This change does not do so.

A plain serde alias is insufficient because it produces a generic duplicate-field error and cannot accept equal dual writes. Use an explicit deserialization/reconciliation representation.

## Result compatibility and observability

The terminal state value is unchanged for existing clients. (The original premise named a terminal state string `timeout`; no such value exists in this codebase. Abandonment surfaces through the existing `killed` state and its terminating signal, and that mapping is left exactly as it was.) When `abandon-job-after` actually causes termination, persisted state and every projection derived from it expose additive fields:

- `abandoned_by: "abandon_job_after"`
- `result_loss: true`

`status`, `list`, completion events, schemas, and skills explain that the additive pair — not the terminal state value — denotes abandonment by this workload control. Historical state files without the additive fields remain readable and do not synthesize unsupported provenance.

## Surface mapping

| Surface | Old | New / retained behavior |
|---|---|---|
| CLI `run` / `create` | `--timeout` | `--abandon-job-after` + `--acknowledge-result-loss`; hidden old flag always errors |
| MCP `run` | `timeout` | `abandon_job_after` + `acknowledge_result_loss=true` |
| HTTP `/exec` | `timeout` | `abandon_job_after` + `acknowledge_result_loss=true` |
| Public Rust `run::RunOpts` | `timeout_ms` | `abandon_job_after_ms` + acknowledgement boolean |
| Public Rust `run::SuperviseOpts` | `timeout_ms` | `abandon_job_after_ms`; internal already-admitted value |
| Public Rust `create::CreateOpts` | `timeout_ms` | `abandon_job_after_ms` + acknowledgement boolean |
| Embedded public request | `timeout_ms` | `abandon_job_after_ms` + acknowledgement boolean |
| Private `_supervise` argv/parser | `--timeout` | unchanged private handoff |
| Persisted metadata | `timeout_ms` | migration release dual-writes equal old/new fields; reads legacy/new/equal dual |
| Terminal result | terminal `state` value | state unchanged; additive abandonment markers on actual deadline termination |
| Observation | `until` | unchanged and non-destructive |

## Verification strategy

Use `abandon_job_after`-prefixed tests across all three integration targets, then run `cargo test abandon_job_after` so CLI, MCP, and HTTP tests execute together. Coverage includes:

- missing/false acknowledgement rejection before job creation;
- hidden CLI legacy flag and structured legacy fields returning actionable migration errors;
- short acknowledged processes that exceed `abandon-job-after`;
- longer processes observed with `until` and left running;
- actual-abandonment markers in state/status/list/completion event and absence when the limit does not fire;
- legacy-only, new-only, equal dual, and unequal dual metadata fixtures;
- start/restart and an old-binary downgrade fixture proving the dual-written limit is retained;
- compiled public Rust API names and the private supervisor handoff/parser remaining mutually consistent;
- grep-style checks that current public docs/skills/examples contain no live old launch syntax outside migration/rejection text.

Archive validation follows canonical requirement consolidation in this proposal. MODIFIED deltas restate complete canonical requirements and all existing scenarios before adding the new behavior.
