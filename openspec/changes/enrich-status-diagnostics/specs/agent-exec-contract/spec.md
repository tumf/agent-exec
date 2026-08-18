## ADDED Requirements

### Requirement: Status execution diagnostics

A successful `status` response SHALL expose execution diagnostics derivable from canonical job metadata, state, and log files. The public schema additions SHALL be optional for backward compatibility; the current implementation SHALL apply these presence rules:

| field | type | presence | source |
|---|---|---|---|
| `command` | `string[]` | always | persisted argv; never shell-expanded |
| `cwd` | `string` | when persisted cwd exists | metadata |
| `tags` | `string[]` | always; `[]` when none | metadata |
| `pid` | integer | when persisted PID exists | state |
| `process_alive` | boolean | according to the liveness table below | best-effort probe |
| `updated_at` | RFC 3339 string | always | state |
| `elapsed_ms` | non-negative integer | non-terminal state with `started_at` | response time minus `started_at` |
| `duration_ms` | non-negative integer | when persisted result duration exists | state result |
| `signal` | string | when persisted result signal exists | state result |
| `logs_drained` | boolean | always | state |
| `stdout_log_path` / `stderr_log_path` | string | always | canonical job directory |
| `stdout_total_bytes` / `stderr_total_bytes` | non-negative integer | always; `0` when missing or unreadable | file metadata size |

Existing fields (`job_id`, `state`, `exit_code`, `created_at`, `started_at`, `finished_at`) SHALL retain their names, types, presence rules, and meanings. The response SHALL NOT expose environment-variable values, stdin content, notification secrets, or shell-expanded command strings.

`process_alive` SHALL be resolved only for persisted `running` state:

| persisted state | PID | probe available | `process_alive` |
|---|---|---|---|
| `running` | present | yes | probe result |
| `running` | absent | — | `false` |
| `running` | present | no | omitted |
| `created`, `exited`, `killed`, `failed` | any | — | omitted |

The probe SHALL NOT run for non-running state because a recorded PID may have been reused. Omission means no live observation was made. The observation is best-effort and same-user scoped; it is not an authoritative liveness guarantee, and platform probes may classify inaccessible processes differently.

PID liveness SHALL be reported separately from persisted `state` and SHALL NOT mutate state during a read-only query. Status SHALL reuse the liveness probe behind the list reconciliation defined in the canonical `agent-exec` specification rather than redefining list state semantics. A stale running job therefore appears as `unknown` in list and as `state="running", process_alive=false` in status.

Log totals SHALL be obtained using bounded file-size metadata queries. Status SHALL NOT read log contents, and its response cost SHALL not grow with log size.

#### Scenario: Running job exposes execution context and live observability

**Given**: a running job has persisted command, cwd, tags, PID, timestamps, and canonical log files
**When**: `agent-exec status <job_id>` is executed
**Then**: the response includes fields required by the presence table
**And**: `elapsed_ms` represents response time minus `started_at`
**And**: `duration_ms` is absent
**And**: `process_alive` reflects the best-effort PID probe
**And**: `state` remains persisted lifecycle state

#### Scenario: List and status correspond for stale running state

**Given**: state records `running` with a PID that is no longer alive
**When**: both `agent-exec list --all` and `agent-exec status <job_id>` are executed
**Then**: list presents the job as `state="unknown"`
**And**: status presents `state="running"` with `process_alive=false`
**And**: neither command rewrites state

#### Scenario: Terminal job reports persisted outcome

**Given**: a terminal job has persisted duration, signal, and log-drain state
**When**: status is executed
**Then**: `duration_ms`, `signal`, and `logs_drained` report persisted values
**And**: `elapsed_ms` and `process_alive` are absent
**And**: a null persisted duration is not computed at read time

#### Scenario: Missing logs have zero observed bytes

**Given**: one or both canonical log files do not exist or cannot be read
**When**: status is executed
**Then**: the corresponding total is `0`
**And**: status remains successful

#### Scenario: Large logs do not increase status read cost

**Given**: canonical logs contain at least 8 MiB of data
**When**: status is executed
**Then**: byte totals equal file metadata sizes
**And**: status does not read log contents into memory

#### Scenario: Sensitive inputs are not disclosed

**Given**: a job was created with environment values, masks, stdin content, or notification configuration
**When**: status is executed
**Then**: none of those values or contents appear
**And**: command remains the persisted argv array

### Requirement: Status diagnostic schema compatibility

Status diagnostics SHALL be introduced as schema `0.3` optional fields. The checked-in JSON Schema and changelog SHALL describe them, and existing fields SHALL remain unchanged. The StatusResponse schema SHALL also match existing created-job behavior by defining required `created_at`, making `started_at` optional, and allowing `state="created"`.

Because `schema_version` is a global contract version, completion and output-match event envelopes SHALL also carry `0.3` without field-shape changes. Tracked source constants, tests, README, and bundled skill references SHALL agree on the version.

#### Scenario: Existing clients ignore enriched status fields

**Given**: a client supports schema major version `0` and ignores unknown optional fields
**When**: it receives a schema `0.3` status response
**Then**: it continues reading existing fields unchanged

#### Scenario: Created status validates against the published schema

**Given**: a job exists in created state without `started_at`
**When**: its status response is validated against the checked-in JSON Schema
**Then**: validation succeeds
**And**: `created_at` and `state="created"` are represented by the schema
