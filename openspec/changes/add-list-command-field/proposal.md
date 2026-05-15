---
change_type: implementation
priority: medium
dependencies: []
references:
  - AGENTS.md
  - src/schema.rs
  - src/list.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-jobstore/spec.md
---

# 変更提案: list の job summary に command を含める

## Problem / Context

`agent-exec list` はエージェントが自分の作業ディレクトリの job を少ない往復で把握するための入口である。現在の `JobSummary` は `job_id`, `short_job_id`, `state`, timestamp, tags を返すが、実行された `command` を返さない。そのためユーザーやエージェントは、複数 job の中から目的の job を見分けるために追加で `status` や job directory の `meta.json` を確認する必要がある。

一方、`meta.json` には既に `command` が保存されており、canonical jobstore spec でも `meta.json.command` は必須である。`list` が既に `meta.json` を読む以上、summary に `command` を含めることは既存データの公開範囲を拡張するだけで、job lifecycle や cwd filtering のコア概念は変えない。

**Change Type**: implementation

## Requested Artifact

implementation

## Proposed Solution

- `src/schema.rs` の `JobSummary` に `command: Vec<String>` を追加する。
- `src/list.rs` で各 job の `meta.command` を `JobSummary.command` にコピーする。
- `list` の JSON payload spec を更新し、各 job summary が保存済み `command` を含むことを MUST とする。
- `tests/integration.rs` に regression coverage を追加し、`agent-exec list` の job entry が実際に実行した argv を返すことを検証する。

## Acceptance Criteria

- `agent-exec list` の `jobs[]` 各要素に `command` が含まれる。
- `command` は対象 job の `meta.json.command` と同じ string array であり、argv の順序を保持する。
- cwd/state/tag/limit filtering と並び順の既存挙動は変わらない。
- stdout は従来どおり 1 つの JSON envelope のみで、診断ログや追加テキストを混ぜない。
- 既存 job で `meta.json.command` が読める限り、`list` は追加の job inspection なしで実行コマンドを判断できる。

## Explicit Completion Conditions

- `src/schema.rs` の `JobSummary` に public JSON field `command` が追加されている。
- `src/list.rs` の `jobs.push(JobSummary { ... })` が `meta.command` 由来の値を設定している。
- `tests/integration.rs` に `list` response の `jobs[].command` を実コマンド配列と照合する integration test がある。
- `openspec/specs/agent-exec/spec.md` の `list の JSON ペイロード` canonical requirement に、archive 後 `command` 必須化が反映される delta がある。
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all` が成功する。

## Out of Scope

- `list` の cwd filter 既定、`--all`, `--cwd`, `--state`, `--tag`, `--limit` の semantics 変更。
- `status`, `tail`, `wait`, `run`, `start` の response shape 変更。
- `meta.json.command` の保存形式変更。
- shell-string mode と argv mode の解釈変更。
- secret masking policy の変更。既存の `meta.json.command` に保存される値をそのまま表示する範囲に限定する。
