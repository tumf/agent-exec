# agent-exec-contract Specification

## Purpose
TBD - created by archiving change define-agent-exec-contract-v0-1. Update Purpose after archive.
## Requirements

### Requirement: CLI サブコマンド構成

`agent-exec` は `schema` サブコマンドを提供しなければならない（MUST）。`schema` は stdout に `type="schema"` の JSON を 1 つ出力しなければならない（MUST）。`schema` の JSON は `schema_format` と `schema` を含み、`schema_format` は `json-schema-draft-07` でなければならない（MUST）。

#### Scenario: schema を取得する

Given `agent-exec schema` を実行する
When コマンドが完了する
Then stdout は `type="schema"` の JSON である
And `schema_format` は `json-schema-draft-07` である
And `schema` は JSON オブジェクトである

### Requirement: ヘルプは英語

`-h`/`--help` は常に有効でなければならない（MUST）。トップレベルおよび各サブコマンドのヘルプ文言は英語でなければならない（MUST）。

#### Scenario: サブコマンドヘルプ
Given `agent-exec run --help` を実行する
When ヘルプが表示される
Then 表示内容は英語である

### Requirement: stdout JSON-only と stderr 分離

すべてのサブコマンドは stdout に JSON オブジェクト 1 つのみを出力しなければならない（MUST）。stderr は診断ログのみに使用しなければならない（MUST）。対話的なプロンプトは行ってはならない（MUST）。

#### Scenario: status の標準出力
Given `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は JSON のみであり、stderr にのみログが出力される

### Requirement: 共通レスポンスエンベロープ

すべての出力 JSON は `schema_version`, `ok`, `type` を含まなければならない（MUST）。`ok=false` の場合は `error` オブジェクトを含まなければならない（MUST）。

#### Scenario: ジョブ未検出
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが完了する
Then stdout は `ok=false` を含む JSON であり、`error` が含まれる

### Requirement: エラーオブジェクト形式

`error` は `code`, `message`, `retryable` を必須フィールドとして持たなければならない（MUST）。

#### Scenario: エラー応答の必須フィールド
Given `agent-exec status <missing_job_id>` を実行する
When コマンドが完了する
Then `error.code` と `error.message` と `error.retryable` が含まれる

### Requirement: 終了コード

成功時は `0`、期待される失敗（対象未検出/バリデーション失敗/I/O など）は `1`、CLI usage エラーは `2` を返さなければならない（MUST）。

#### Scenario: 期待される失敗の終了コード
Given 存在しない `job_id` に対して `agent-exec status <job_id>` を実行する
When コマンドが終了する
Then 終了コードは `1` である


#

## Requirements

### Requirement: schema_version のバージョニングポリシー

`schema_version` は `"MAJOR.MINOR"` 形式の文字列でなければならない（MUST）。両セグメントは非負整数であり、先頭ゼロを含んではならない（MUST NOT）。

後方互換のあるフィールド追加（optional field の追加、enum variant の追加）は MINOR bump で行わなければならない（MUST）。既存フィールドの削除、型変更、意味変更、required 化は MAJOR bump を要する（MUST）。

`schema_version` が bump されるとき、repository tracked changelog artifact に対応する `## schema <version>` セクションを追加しなければならない（MUST）。

クライアント／エージェントは MAJOR が一致する JSON を解釈できなければならない（MUST）。未知の optional field を受け取った場合はそれを無視できなければならない（forward compatibility、MUST）。MAJOR 不一致の場合はエラー扱いとしてよい（MAY）。

#### Scenario: adding an optional field bumps MINOR

**Given**: canonical `schema_version = "0.1"`
**When**: a new optional field is added to `RunData`
**Then**: the next `schema_version` is `"0.2"` with a `## schema 0.2` entry in the repository changelog

#### Scenario: removing a field bumps MAJOR

**Given**: canonical `schema_version = "0.9"`
**When**: an existing field is removed from `RunData`
**Then**: the next `schema_version` is `"1.0"` with a `## schema 1.0` entry in the repository changelog

#### Scenario: MCP run adds optional notification object

**Given**: canonical `schema_version = "0.1"`
**When**: MCP `RunData` gains an optional structured `notification` field
**Then**: the response schema advances to `"0.2"`
**And**: the repository contains a `## schema 0.2` changelog entry describing the optional field
**And**: existing field names, types, and meanings remain unchanged
**And**: clients that ignore unknown optional fields remain compatible

### Requirement: エラーレスポンスの構造化 details

エラーレスポンスの `error` オブジェクトは `code`・`message`・`retryable` に加え、任意の構造化補足情報を `details`（JSON object）として含めてよい（MAY）。`details` は安定したキー集合を持つ error code ごとにスキーマを規定する（MUST）。

`error.code = "ambiguous_job_id"` の場合、`details` は以下を必ず含めなければならない（MUST）:
- `candidates`: 衝突した完全な `job_id` の配列。最大 20 件まで。
- `truncated`: 候補が 20 件を超えたときに `true`、そうでなければ `false`。

#### Scenario: ambiguous_job_id returns structured candidates

**Given**: 2 jobs share a common prefix
**When**: `agent-exec status <shared-prefix>` is executed
**Then**: the response includes `error.code="ambiguous_job_id"`
**And**: `error.details.candidates` is an array of length ≥ 2
**And**: `error.details.truncated` is `false`

#### Scenario: ambiguous_job_id truncates large candidate sets

**Given**: 25 jobs share a common prefix
**When**: `agent-exec status <shared-prefix>` is executed
**Then**: `error.details.candidates` contains 20 entries
**And**: `error.details.truncated` is `true`

### Requirement: Embedded typed managed-job API

The Rust crate SHALL expose a synchronous typed API for `run`, `status`, `tail`, `list`, and `kill` that operates on an explicit jobs root without invoking the public `agent-exec` CLI, parsing command JSON, or writing command responses to stdout. The API SHALL return domain types for job identity, state, output ranges and totals, list summaries, signal observations, and structured error categories. It SHALL preserve the same jobstore, lookup, tag, observation, timeout, signal, logging, masking, notification, and process-tree semantics used by the standalone CLI.

#### Scenario: Consumer manages a job without CLI JSON

**Given**: a Rust consumer links the crate, configures an isolated jobs root, and installs supervisor startup delegation
**When**: it calls typed run, status, tail, list, and kill methods
**Then**: each operation returns Rust domain data without spawning the public CLI or parsing/printing a JSON response
**And**: the job remains visible through the same persisted jobstore contract

#### Scenario: Embedded list filters by recovery tags

**Given**: multiple jobs exist under one explicit root with different persisted tags
**When**: the consumer calls typed list with repeated tag filters and all-directory scope
**Then**: the result contains only jobs satisfying every tag pattern and preserves current ordering, truncation, skipped-count, state, and exit-code semantics

#### Scenario: Embedded errors are machine-classifiable

**Given**: a consumer requests a missing or ambiguous job, submits invalid input, or encounters supervisor launch or storage failure
**When**: the typed operation returns an error
**Then**: the consumer can distinguish the stable error category and retryability without parsing human-readable message text

### Requirement: Embedding supervisor startup delegation

The crate SHALL expose a startup delegation entrypoint for embedding binaries. It SHALL claim an invocation only when the exact reserved supervisor marker occurs at `argv[1]`, return immediately without altering ordinary consumer startup otherwise, and execute the same detached supervision implementation used by the standalone CLI for a valid invocation. Once the marker is claimed, missing, duplicate, malformed, or unexpected generated arguments SHALL fail closed without entering normal consumer handling. The marker is a reserved dispatch token rather than an authentication boundary; delegated supervision SHALL validate the explicit root and job identity against pre-created metadata before acknowledging startup. The embedded client SHALL default supervision to the current executable and SHALL allow an explicit trusted supervisor executable override.

#### Scenario: Ordinary consumer invocation is untouched

**Given**: the embedding binary starts with its normal application arguments
**When**: it calls the delegation entrypoint before its own argument parser
**Then**: delegation reports that the process is not a supervisor invocation and the consumer receives its original arguments unchanged

#### Scenario: Reserved invocation runs supervision

**Given**: the embedded client re-executes the configured binary with a valid reserved supervisor invocation
**When**: startup delegation examines the invocation
**Then**: it runs supervision to terminal state without entering the consumer's normal command handling

#### Scenario: Reserved marker claims the invocation

**Given**: the exact reserved supervisor marker occurs at `argv[1]`
**When**: required generated arguments are missing, duplicated, malformed, or followed by unexpected trailing arguments
**Then**: delegation returns a bounded error and does not pass the invocation to normal consumer argument handling

#### Scenario: Malformed reserved invocation fails closed

**Given**: the process contains the reserved supervisor marker but required identity, root, or execution arguments are malformed or missing
**When**: startup delegation examines it
**Then**: it returns a bounded error and does not run either supervision or normal consumer command handling

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
