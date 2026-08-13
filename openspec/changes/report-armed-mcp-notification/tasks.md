## Implementation Tasks

- [x] Extend MCP `run` with optional launch-time completion sinks and validate them before creating or launching a workload. Completion condition: valid command/file sink input reaches canonical notification persistence; invalid or conflicting input returns a protocol-safe error with no workload process. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_persists_completion_sink_before_launch`)

- [x] Add the optional structured `notification` response to successful MCP run results. Completion condition: a still-running job with persisted completion metadata returns `state="armed"`, generic sink classifications, `polling_required=false`, and the no-poll message; runs without a sink and failed admissions never claim armed state. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_run_reports_only_persisted_notification_as_armed`)

- [x] Keep the MCP contract client-independent. Completion condition: core source and schemas contain no specific client, session, chat, or originating-client assumptions; multiple generic sink classes produce the same lifecycle semantics. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_notification_hint_is_client_independent`)

- [x] Verify agents can stop observation based only on the response contract. Completion condition: MCP integration fixtures assert `polling_required=false` and the no-poll message for armed jobs, while unarmed jobs omit that claim and leave explicit observation available. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test mcp_integration mcp_armed_response_explains_next_action`)

- [x] Apply and document the backward-compatible schema change. Completion condition: response types, checked-in JSON schema, schema command output, contract documentation, and version/changelog evidence agree on the optional notification field and minor schema version; existing fields retain their meanings. (verification-id: mcp-armed-notification) (verification: integration - `cargo test --test integration schema_command_matches_checked_in_schema`)

## Future Work

- Add optional host-specific adapters separately; they must translate their destination identity into generic MCP sink input without changing core response semantics.
- Add durable retry delivery separately if best-effort completion sinks prove insufficient.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate report-armed-mcp-notification --archive-gate`.

- evidence: `cargo test --all` passed (163 unit + 272 integration + 24 mcp + 24 serve + 14 embedded + 4 opencode + 2 embedded-consumer + 1 doctest; 2 pre-existing heavy tests ignored)
- evidence: `prek run -a` passed (trailing-whitespace, end-of-file-fixer, check-toml, check-yaml, cargo fmt --check, cargo clippy -D warnings, cargo test --all)
- evidence: `cflx openspec validate report-armed-mcp-notification --strict` passed

## Acceptance Repair Notes (attempt 1)

Investigated defect: the `schema_version` 0.1 -> 0.2 bump required by task 5 was applied to
`src/schema.rs`, `schema/agent-exec.schema.json`, `CHANGELOG.md`, `README.md`,
`skills/agent-exec/references/cli-contract.md`, and the `site/docs` pages, but it missed the
shipped `job.finished` example in `skills/agent-exec/references/completion-events.md:9`. That
file is embedded into the binary and written out by `agent-exec skill install`
(`tests/integration.rs:2953`), so agents were handed a completion-event contract claiming
`"schema_version": "0.1"` while `src/run.rs:2067` emits `SCHEMA_VERSION` = `"0.2"`. Verified
against a real run: `agent-exec run --notify-file <path> -- sh -c 'echo hi'` wrote
`{"schema_version":"0.2","event_type":"job.finished",...}`. This contradicts task 5's completion
condition that contract documentation agree on the minor schema version, and the
`agent-exec-contract` schema-version policy requirement.

Nothing enforced that agreement, so the drift was invisible to `cargo test --all` and `prek run -a`.
The repair adds the missing enforcement rather than only correcting the one file.

Secondary precision fix in the same schema-contract documentation: the `## schema 0.2` CHANGELOG
entry described `notification` presence conditions without saying which surface emits it. Only
MCP `run` populates the field; `src/run.rs:1028`, `src/start.rs:168`, `src/restart.rs:174`, and
`src/serve.rs:385`/`:453` all emit `None`. The entry now says so.

## Current Acceptance Follow-up
- attempt: 1
- [x] Investigate acceptance failure and apply the required fix
  evidence: root cause = the schema 0.1->0.2 bump missed skills/agent-exec/references/completion-events.md:9, shipping a job.finished example that contradicts the binary's actual `"schema_version":"0.2"` output from src/run.rs:2067
  evidence: reproduced against the built binary - `agent-exec run --notify-file <path> -- sh -c 'echo hi'` wrote `{"schema_version":"0.2","event_type":"job.finished",...}` while the shipped doc said "0.1"
  evidence: skills/agent-exec/references/completion-events.md:9 now documents `"schema_version": "0.2"`, agreeing with SCHEMA_VERSION, schema/agent-exec.schema.json, README.md, and the site/docs pages
  evidence: tests/integration.rs `documented_schema_version_examples_track_schema_version` scans README.md, CHANGELOG.md, docs/, skills/, and site/ for `"schema_version": "<v>"` JSON literals and fails on any value != SCHEMA_VERSION
  evidence: guard proven to catch the defect - reverting the doc to "0.1" fails with `...completion-events.md documents schema_version "0.1" but the binary emits "0.2"`, and it passes on the fixed doc
  evidence: CHANGELOG.md `## schema 0.2` now states `notification` is produced by MCP `run` and omitted by CLI/HTTP responses (src/run.rs:1028, src/start.rs:168, src/restart.rs:174, src/serve.rs:385)
  evidence: `prek run -a` passed after the repair (trailing-whitespace, end-of-file-fixer, check-toml, check-yaml, cargo fmt --check, cargo clippy -D warnings, cargo test --all)
  evidence: `cflx openspec validate report-armed-mcp-notification --strict` and `--archive-gate` both passed after the repair
