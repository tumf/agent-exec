---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/mcp.rs
  - src/run.rs
  - tests/mcp_integration.rs
  - openspec/specs/agent-exec-mcp/spec.md
  - openspec/specs/agent-exec-run/spec.md
verifications:
  - id: mcp-stdin-tests
    requirement: MCP run passes inline and file-backed input through the canonical managed-job stdin lifecycle and rejects invalid input safely
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: cargo test output for MCP integration tests plus fmt and clippy results
    rerun: cargo test --test mcp_integration && cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Add stdin input to MCP run

**Change Type**: implementation

## Problem / Context

The CLI `run` path already supports inline and file-backed stdin through `StdinSource`, bounded materialization into the job directory, persisted `meta.json.stdin_file`, and detached supervisor handoff. The MCP `run` schema currently exposes only `command`, `cwd`, `env`, `timeout`, and `until`, so MCP clients cannot use that canonical stdin lifecycle. Because MCP stdio is the protocol transport, it cannot also be treated as the managed command's stdin stream.

## Proposed Solution

Extend the MCP `run` tool with two optional, mutually exclusive fields:

- `stdin`: an inline UTF-8 string supplied in the MCP tool arguments.
- `stdin_file`: a path readable by the MCP server process and resolved by the existing file-backed stdin path.

Map these fields to the existing `StdinSource::Inline` and `StdinSource::File` variants and pass the result through `run::run_response`. Reuse the existing default stdin size limit, job-local `stdin.bin` materialization, persisted metadata, detached supervisor behavior, and stable domain errors. Do not interpret `stdin: "-"` as MCP transport input; MCP has no separate caller stdin stream available for that meaning.

This remains one proposal because schema exposure, canonical lifecycle wiring, and integration tests are inseparable: accepting either field without runtime handoff would create a misleading no-op API.

## Acceptance Criteria

- MCP `tools/list` describes optional `stdin` and `stdin_file` string fields for `run`.
- Calling MCP `run` with `stdin` sends the exact UTF-8 bytes to the managed child and returns them through normal output observation when the child echoes stdin.
- Calling MCP `run` with `stdin_file` copies the server-local file into the job's canonical stdin materialization before launch, so later source-file changes cannot alter the running job input.
- Jobs started with either input persist `meta.json.stdin_file` using the existing canonical job-local stdin file contract.
- Supplying both fields returns a protocol-safe error and creates no job.
- A missing, unreadable, or oversized `stdin_file`, and oversized inline input, fails before child launch using the existing stable error mapping where available.
- Omitting both fields preserves current MCP behavior: the child receives null stdin and the MCP protocol stream is never consumed as job input.
- Existing MCP `command`, `cwd`, `env`, `timeout`, and `until` behavior remains backward compatible.

## Explicit Completion Conditions

- `src/mcp.rs` exposes and validates both fields, maps them to existing stdin source types, and supplies the existing default byte limit to `RunOpts`.
- No second stdin storage or child-process piping implementation is introduced outside the canonical `src/run.rs` lifecycle.
- `tests/mcp_integration.rs` executes real MCP tool calls covering inline success, file success, mutual-exclusion rejection without job creation, omitted-input compatibility, and representative file/size errors.
- `cargo test --test mcp_integration`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets --all-features -- -D warnings` pass.

## Out of Scope

- Streaming or interactive stdin after a job has started.
- Reading managed-command stdin from the MCP server's stdio transport.
- Uploading a client-local file to a remote MCP host by path alone.
- Adding configurable MCP stdin size limits or binary/base64 input fields.
- Extending the HTTP `serve` API in the same change.
