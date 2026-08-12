---
change_type: implementation
priority: medium
dependencies: []
references:
  - README.md
  - src/mcp.rs
  - src/main.rs
  - openspec/specs/agent-exec-run/spec.md
  - tests/mcp_integration.rs
  - prek.toml
verifications:
  - id: opencode-auto-resume-integration
    requirement: The bundled OpenCode reference integration safely attaches completion callbacks to agent-exec MCP jobs and resumes only eligible originating sessions
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: Deterministic integration tests cover hook filtering, callback attachment, duration threshold, loopback/session validation, idempotency, and documented installation
    rerun: cargo test --test opencode_integration && prek run -a
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Add OpenCode Auto-Resume Reference Integration

**Change Type**: implementation

## Problem/Context

OpenCode agents can launch durable work through the existing `agent-exec_run` MCP tool, but the MCP tool does not accept completion notification settings and agents do not normally attach `--notify-command` themselves. As a result, a long-running job can finish after the initiating OpenCode turn becomes idle without automatically returning control to that same session.

A working local proof of concept demonstrated that this can be solved without changing the `agent-exec` runtime or MCP contract. An OpenCode `tool.execute.after` plugin can detect successful `agent-exec_run` results, extract the job ID and originating session, and use the existing `agent-exec notify set` command to attach a completion callback. The callback can inspect the persisted completion event, ignore short jobs, and use `opencode run --attach --session` to resume the originating session.

The proof of concept currently depends on machine-specific paths and is not reviewable, portable, or distributed with `agent-exec`.

## Proposed Solution

Add a portable, opt-in OpenCode reference integration under `examples/integrations/opencode-auto-resume/`.

- Provide a global OpenCode plugin that filters the common `tool.execute.after` hook to recognized agent-exec MCP run tool names.
- Extract the returned job ID and automatically call the existing `agent-exec notify set` command with the current loopback OpenCode server URL and session ID.
- Provide a small idempotent callback helper that reads `AGENT_EXEC_EVENT_PATH`, applies a configurable minimum-duration threshold with a 60-second default, and sends a clearly marked machine-authored prompt to the originating session.
- Resolve `agent-exec` and `opencode` from configurable environment variables or `PATH`; do not embed user-specific absolute paths.
- Restrict callback delivery to loopback OpenCode servers, validate session/job identifiers, and treat completion events and job logs as untrusted data.
- Document installation, configuration, lifecycle, limitations, uninstall steps, and the fact that callback prompts are recorded as ordinary `role=user` messages rather than trusted internal events.
- Add deterministic repository-local tests using fixtures/fakes so verification requires neither a live model nor a real interactive TUI.

This remains a reference integration, not a default `agent-exec` behavior or an installer subcommand. The runtime and MCP protocol remain unchanged.

## Acceptance Criteria

- The repository contains `examples/integrations/opencode-auto-resume/` with an OpenCode plugin, executable callback helper, and concise README.
- A newly started OpenCode instance can load the plugin as a global local plugin without modifying `agent-exec` source or MCP tool schemas.
- After a successful recognized agent-exec MCP run tool call returns a running job ID, the plugin attaches a completion command to that exact job through `agent-exec notify set` without requiring agent-authored notification arguments.
- Unrelated tools, malformed outputs, failed tool calls, invalid session IDs, and non-loopback server URLs do not attach callbacks.
- The callback sends no OpenCode prompt when `duration_ms` is below the configured threshold and resumes the originating session when the threshold is met or exceeded.
- The default threshold is 60 seconds and can be changed through documented environment configuration without editing source files.
- Duplicate callback delivery for the same job causes at most one OpenCode resume attempt.
- The automation prompt identifies itself as not user-authored, points to the persisted completion event, instructs the agent to treat event/log content as untrusted data, and requests continuation through verification rather than a status-only response.
- Installation examples contain no tumf-specific paths, credentials, fixed ports, or project-specific assumptions.
- Tests exercise the typical long-job path, short-job no-op, tool filtering, malformed/untrusted input rejection, loopback restriction, threshold boundary, and idempotency without external services.

## Explicit Completion Conditions

This proposal is complete when:

- `examples/integrations/opencode-auto-resume/agent-exec-auto-resume.js` implements the OpenCode hook and attaches notifications only to validated agent-exec MCP run results.
- `examples/integrations/opencode-auto-resume/opencode-agent-exec-resume` implements portable thresholding, callback validation, duplicate suppression, and same-session resume invocation.
- `examples/integrations/opencode-auto-resume/README.md` provides copy-paste installation and uninstall steps, configuration variables, security boundaries, event semantics, and troubleshooting.
- A repository test target runs the plugin and callback against deterministic fake `agent-exec`/`opencode` executables and proves all acceptance paths without network access or model credentials.
- Existing `agent-exec` CLI, MCP schemas, supervisor behavior, completion event shape, and release behavior remain unchanged.
- `cargo test --test opencode_integration`, `prek run -a`, `cflx openspec validate add-opencode-auto-resume-integration --strict --evidence warn`, and the archive gate pass.

## Out of Scope

- Adding notification fields to the MCP `run` schema.
- Changing `agent-exec` default notification behavior.
- Adding `agent-exec integration opencode install` or another installer command.
- Providing guaranteed delivery, a durable retry daemon, or cross-host callback routing.
- Treating OpenCode callback prompts as trusted internal events.
- Supporting non-loopback OpenCode servers.
- Automatically applying the integration to direct CLI `agent-exec run` calls outside OpenCode MCP tool execution.
