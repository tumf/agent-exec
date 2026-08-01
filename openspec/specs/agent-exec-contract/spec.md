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

`schema_version` が bump されるとき、リポジトリ直下の `CHANGELOG.md` に対応する `## schema <version>` セクションを追加しなければならない（MUST）。

クライアント／エージェントは MAJOR が一致する JSON を解釈できなければならない（MUST）。未知の optional field を受け取った場合はそれを無視できなければならない（forward compatibility、MUST）。MAJOR 不一致の場合はエラー扱いとしてよい（MAY）。

#### Scenario: adding an optional field bumps MINOR

**Given**: canonical `schema_version = "0.1"`
**When**: a new optional field is added to `RunData`
**Then**: the next `schema_version` is `"0.2"` with a `## schema 0.2` entry in CHANGELOG.md

#### Scenario: removing a field bumps MAJOR

**Given**: canonical `schema_version = "0.9"`
**When**: an existing field is removed from `RunData`
**Then**: the next `schema_version` is `"1.0"` with a `## schema 1.0` entry in CHANGELOG.md

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
