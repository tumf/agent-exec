---
change_type: implementation
priority: high
dependencies: []
references:
  - src/mcp.rs
  - src/schema.rs
  - tests/mcp_integration.rs
  - tests/opencode_integration.rs
  - examples/integrations/opencode-auto-resume/agent-exec-auto-resume.js
  - openspec/changes/archive/2026-08-13-add-opencode-auto-resume-integration
verifications:
  - id: mcp-armed-notification
    requirement: MCP run persists a supplied completion sink before launch and truthfully tells the calling agent when completion notification is armed and polling is unnecessary
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: MCP integration tests cover launch-time sink persistence, truthful response hints, absent and invalid sinks, and backward compatibility
    rerun: cargo test --test mcp_integration && prek run -a
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Report Armed MCP Completion Notification

**Change Type**: implementation

## Problem/Context

Any MCP host can launch a long `agent-exec` job. Today the `run` response reports only that the job remains running; it does not say whether a completion sink was configured or whether polling is required. Agents therefore follow generic long-job guidance and repeatedly call `wait`, wasting turns and compute even when the caller has already supplied a completion path.

Completion-sink admission and the agent-facing hint need to be part of the client-independent MCP `run` contract. Each host may choose how to construct a sink, but core behavior must not name or depend on a specific client.

## Proposed Solution

- Add an optional launch-time completion command sink to MCP `run`, using the canonical `run` notification persistence path before supervisor launch.
- When `run` returns `state="running"` and the completion sink was persisted, include a structured `notification` object with `state="armed"`, `polling_required=false`, sink classification, and a short message telling the agent that completion will be delivered through the configured sink.
- Do not claim `armed` when no sink was supplied or sink persistence/launch failed.
- Keep `wait`, `status`, and `tail` available for explicit progress requests and diagnosis, but instruct agents not to poll after an armed response.
- Apply the repository's schema-version policy for the backward-compatible response-field addition and document the new minor schema contract.

## Acceptance Criteria

- MCP `run` accepts a launch-time completion command sink and persists it before the managed workload starts.
- A successful non-terminal MCP run with that sink returns a machine-readable `notification` object containing `state="armed"` and `polling_required=false`.
- The response message clearly says completion will be delivered through the configured sink and repeated `wait`/`status`/`tail` calls are unnecessary.
- MCP runs without a completion sink do not claim notification is armed.
- Invalid notification input fails before workload launch and does not create a misleading armed response.
- Any MCP client can supply the same sink input and receive the same notification status without client-specific core behavior.
- A client that receives `notification.state="armed"` can stop observing immediately and rely on its configured sink.
- Existing CLI notification behavior and explicit observation commands remain available.

## Explicit Completion Conditions

- `src/mcp.rs` maps the new MCP input into the canonical run notification field and returns the structured hint only from persisted notification state.
- `src/schema.rs` and generated/documented schema artifacts represent the optional notification response without changing existing field meanings.
- Focused MCP tests fail if the response says armed without persisted sink metadata.
- `cargo test --test mcp_integration`, `prek run -a`, strict validation, and archive-gate validation pass.

## Out of Scope

- Guaranteed or cross-host notification delivery.
- Removing `wait`, `status`, or `tail`.
- Automatically inventing or attaching a sink for clients that do not supply one.
- Treating callback messages or job output as trusted input.
- Adding a general workflow engine to `agent-exec`.
