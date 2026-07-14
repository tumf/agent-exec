## Implementation Tasks

- [x] Add the official Rust MCP SDK with only the stdio server/tool features required by this change. Completion condition: `Cargo.toml` resolves a pinned compatible MCP SDK, `cargo build` compiles without adding a second async runtime or custom JSON-RPC framing. verification: unit - `cargo build` completes and `Cargo.lock` records the dependency.

- [x] Extract returnable response constructors for canonical run, status, tail, bounded wait, and kill operations. Completion condition: `src/run.rs`, `src/status.rs`, `src/tail.rs`, `src/wait.rs`, and `src/kill.rs` can produce existing typed `Response<T>` envelopes without stdout writes; CLI execution paths retain one-envelope JSON-only stdout. verification: integration - existing CLI tests in `tests/integration.rs` pass unchanged for representative run/status/tail/wait/kill paths.

- [x] Add `agent-exec mcp [--root <PATH>]` and a stdio MCP server module registering only `run`, `status`, `tail`, `wait`, and `kill`. Completion condition: `agent-exec mcp --help` documents the command in English; MCP initialization and tools-list complete over stdio; stdout contains protocol messages only. verification: integration - a compiled-binary stdio test sends initialize and tools/list requests, asserts the five tool names, and rejects any non-JSON-RPC stdout.

- [x] Implement MCP `run` through the canonical persisted job and supervisor path. Completion condition: valid `command`, optional `cwd`, string-valued `env`, second-based `timeout`, and bounded `until` are validated then passed to shared run behavior; results preserve the existing `type="run"` envelope including job ID, state, inline streams, ranges, byte totals, and log paths. verification: integration - an MCP call to `run` for a deterministic command returns a valid run envelope, persists `meta.json`, and its job ID is observable by CLI `status`.

- [x] Implement MCP observation tools with canonical job-ID resolution and bounded controls. Completion condition: `status` uses shared state lookup; `tail` honors caller `lines`/`max_bytes` with CLI defaults; `wait` defaults to 30 seconds and never exposes unbounded waiting; returned terminal and deadline shapes match CLI response semantics. verification: integration - a running job is observed through all three MCP tools, a short `wait` returns a non-terminal state without exit code while the job remains live, and a later observation reaches terminal state.

- [x] Implement explicit-only MCP cancellation. Completion condition: MCP `kill(job_id)` maps to canonical TERM kill behavior and no other MCP path sends a signal due to disconnect, bounded wait expiration, malformed input, or tool errors. verification: integration - an explicitly killed job reaches killed state; separate jobs remain running after client transport closure and after bounded wait expiry.

- [x] Map MCP trust-boundary failures safely. Completion condition: empty commands, invalid env values, invalid durations, and unknown/ambiguous job IDs do not spawn unintended jobs; domain failures retain stable existing `error.code` envelopes and malformed parameters remain protocol-safe. verification: unit - parameter validation tests cover invalid inputs; integration - MCP calls assert no new job directory for rejected run and `job_not_found` for missing IDs.

- [x] Preserve CLI and HTTP `serve` compatibility while sharing response construction where practical. Completion condition: MCP does not require or start `serve`; existing endpoints, loopback default, non-loopback guard, and token policy remain behaviorally unchanged. verification: integration - existing CLI and serve tests pass, including non-loopback/token tests in `src/serve.rs` and `tests/integration.rs`.

- [x] Update the embedded agent skill for MCP-first managed-job operation. Completion condition: `skills/agent-exec/SKILL.md` gives Hermes Native MCP configuration, MCP-first launch guidance for long/uncertain commands, job-ID follow-up rules, CLI fallback, short synchronous shell exception, and explicit-user-cancellation-only kill rule. verification: manual - review the skill against `openspec/changes/add-mcp-managed-jobs/specs/agent-exec-skills/spec.md`; integration - embedded skill installation test confirms updated `SKILL.md` is installed.

- [x] Run repository quality gates. Completion condition: formatting, lint, and tests pass. (verification: integration - run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.)

## Future Work

- Expand MCP `run` only after a client workflow requires additional CLI definition options and defines their persistence, secret handling, and test coverage.
- Consider a network MCP transport only as a separate security-reviewed change; preserve loopback and authentication defaults.
- Confirm Hermes Native MCP configuration in the consuming Hermes repository or runtime after release.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-mcp-managed-jobs --archive-gate`.

## Acceptance #1 Failure Follow-up
- [x] `src/mcp.rs:132-136` は env key を検証せず `KEY=VALUE` 化するため、空キーや `=` を含むキーが trust boundary を通過する。`tasks.md:15` の invalid env validation 完了主張を満たさない。job 作成前に env key を検証し、拒否時に job directory が作られない unit/integration test を追加すること。
- [x] `tasks.md` の active tasks は全て `[x]` だが、完了条件の統合テスト証拠が不足している。`tests/mcp_integration.rs:109-137` は disconnect 後の生存・別接続からの再観測・`wait(until=1)` の期限到達後も自然終了することを検証せず、`proposal.md:63`、`specs/agent-exec-mcp/spec.md:42-48,58-67,80-85` を満たさない。これらのライフサイクル統合テストを追加すること。
- [x] `tests/mcp_integration.rs:125-133` は wait を `until=0` で呼び、tail の range・total fields・実際の bounds を検証しない。`specs/agent-exec-mcp/spec.md:38-56` の bounded wait と tail bounds シナリオを実行する統合テストを追加すること。
- [x] `tests/mcp_integration.rs:139-152` は empty command のみを拒否検証し、unknown/ambiguous ID、invalid_state、invalid duration/env、malformed parameter shape を検証しない。`tasks.md:15` と `specs/agent-exec-mcp/spec.md:87-96` に従い、stable domain error envelope と非生成・非cancelを統合テストで確認すること。
- [x] `tests/mcp_integration.rs:96-106` は MCP run envelope の `state`、stderr、ranges、byte totals、log paths と job logs を検証しておらず、`specs/agent-exec-mcp/spec.md:19-27` および `tasks.md:9` の証拠が不足する。全必須フィールドとログ存在を assertion すること。
- [x] 実コミット経路の pre-commit hook を `prek run -a` で実行し、archive commitability を確認すること。検証定義は `prek.toml` にある。

## Acceptance #2 Resolution

実コミット経路の behavior-bearing follow-up に `(verification: integration - `prek run -a` job `c0d28fbe557f644d650fe96e2e4761b2` succeeds.)` を追記した。archive gate は上記の `## Final Validation` で再実行する。

## Acceptance #3 Failure Follow-up
- [x] 実コミット経路の archive gate が指摘した verification note を、過去の job ID 参照から repository-verifiable な証拠へ修正すること。実コミット経路の検証定義は `prek.toml` にあり、MCP統合テストは `tests/mcp_integration.rs` にある。(verification: integration - tests/mcp_integration.rs; command: prek run -a)

## Final Validation Notes

Archive-gate の verification-note 指摘は、上記 task の repository paths と runnable `prek` command で解消した。archive gate は `## Final Validation` のコマンドで確認する。

## Final Validation Notes

実コミット経路の verification note は `(verification: integration - tests/mcp_integration.rs; command: prek run -a)` に修正済み。archive gate は `## Final Validation` のコマンドで確認する。
