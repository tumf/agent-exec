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
    evidence: MCP and OpenCode integration tests cover launch-time sink persistence, truthful response hints, callback failure, and same-session continuation
    rerun: cargo test --test mcp_integration && cargo test --test opencode_integration && prek run -a
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Report Armed MCP Completion Notification

**Change Type**: implementation

## Problem/Context

The OpenCode reference integration currently observes a successful `agent-exec_run` result and attaches a callback afterward with `agent-exec notify set`. The callback works, but the MCP response cannot truthfully tell the agent that notification is armed because registration happens only after the response exists. Agents therefore follow generic long-job guidance and repeatedly call `wait`, wasting turns and compute.

OpenCode's `tool.execute.after` result mutation is not a reliable communication channel. The completion sink and agent-facing hint need to be part of the MCP `run` transaction itself.

## Proposed Solution

- Add an optional launch-time completion command sink to MCP `run`, using the canonical `run` notification persistence path before supervisor launch.
- Update the OpenCode plugin to inject its validated loopback/session callback through `tool.execute.before` instead of attaching it after the run response.
- When `run` returns `state="running"` and the completion sink was persisted, include a structured `notification` object with `state="armed"`, `polling_required=false`, delivery classification, and a short message telling the agent that completion will resume the originating session.
- Do not claim `armed` when no sink was supplied or sink persistence/launch failed.
- Keep `wait`, `status`, and `tail` available for explicit progress requests and diagnosis, but instruct agents not to poll after an armed response.
- Apply the repository's schema-version policy for the backward-compatible response-field addition and document the new minor schema contract.

## Acceptance Criteria

- MCP `run` accepts a launch-time completion command sink and persists it before the managed workload starts.
- A successful non-terminal MCP run with that sink returns a machine-readable `notification` object containing `state="armed"` and `polling_required=false`.
- The response message clearly says completion will notify/resume the originating session and repeated `wait`/`status`/`tail` calls are unnecessary.
- MCP runs without a completion sink do not claim notification is armed.
- Invalid notification input fails before workload launch and does not create a misleading armed response.
- The OpenCode plugin validates its server/session inputs and injects the callback before execution; it no longer depends on post-response `notify set` for the normal path.
- A long OpenCode-launched job resumes the same session at completion, while the initiating turn can stop immediately after reading the armed response.
- Existing CLI notification behavior and explicit observation commands remain available.

## Explicit Completion Conditions

- `src/mcp.rs` maps the new MCP input into the canonical run notification field and returns the structured hint only from persisted notification state.
- `src/schema.rs` and generated/documented schema artifacts represent the optional notification response without changing existing field meanings.
- The OpenCode integration uses `tool.execute.before` for launch-time callback injection and preserves loopback, identifier, quoting, best-effort delivery, and untrusted-event boundaries.
- Focused MCP tests fail if the response says armed without persisted sink metadata.
- Focused OpenCode tests fail if the plugin falls back to repeated observation or post-response registration on the normal path.
- `cargo test --test mcp_integration`, `cargo test --test opencode_integration`, `prek run -a`, strict validation, and archive-gate validation pass.

## Out of Scope

- Guaranteed or cross-host notification delivery.
- Removing `wait`, `status`, or `tail`.
- Automatically attaching notifications for non-OpenCode clients that do not supply a sink.
- Treating callback messages or job output as trusted input.
- Adding a general workflow engine to `agent-exec`.
