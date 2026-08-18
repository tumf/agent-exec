---
change_type: implementation
priority: high
dependencies: []
references:
  - skills/agent-exec/scripts/hermes-notify-hook
  - skills/agent-exec/references/hermes.md
  - openspec/specs/agent-exec-mcp/spec.md
verifications:
  - id: hermes-hook-tests
    requirement: The Hermes completion helper delivers through an explicit request-scoped target and fails closed without one
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: focused shell helper test output plus syntax and documentation checks
    rerun: cargo test --test integration -- hermes_notify_hook notify_failure_does_not_change_job_state && cargo test --test mcp_integration mcp_run_reports_only_persisted_notification_as_armed && bash -n skills/agent-exec/scripts/hermes-notify-hook
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Fix Hermes completion hook delivery

**Change Type**: implementation

## Problem / Context

MCP `run` already accepts and persists launch-time completion sinks, and its response can truthfully report an armed sink. The tracked Hermes helper is not usable with the current Hermes CLI: it invokes the nonexistent `hermes notify` command, starts an unnecessary LLM callback, and documents routing variables as managed-child `env` even though completion command sinks do not inherit that job environment.

The result is a sink that can be armed successfully but cannot deliver to the originating Slack or Telegram destination.

## Proposed Solution

Keep the existing generic MCP completion-sink contract unchanged. Repair only the Hermes adapter and guidance:

- require one explicit request-scoped `HERMES_NOTIFY_TARGET` in `platform:chat_id[:thread_id]` form;
- pass that target as an assignment inside the persisted `notify_command`, not through managed-child `env`;
- invoke `hermes send --quiet --to "$HERMES_NOTIFY_TARGET"` with a short message containing only the canonical job ID and completion event path;
- fail closed when the target or Hermes binary is unavailable;
- add a repository-local fake-Hermes test proving exact argv, missing-target rejection, and non-LLM delivery;
- update the Hermes reference to prefer the armed MCP sink and retain the single-watcher path only as a fallback when no sink is persisted.

No Hermes-specific routing logic enters the Rust crate.

## Acceptance Criteria

- The helper never calls `hermes notify` or starts an LLM turn.
- The helper requires `HERMES_NOTIFY_TARGET` and never guesses or caches a destination.
- A fake Hermes executable observes exact `send --quiet --to <target> <message>` arguments.
- The message includes `AGENT_EXEC_JOB_ID` and `AGENT_EXEC_EVENT_PATH` and excludes command output and credentials.
- Missing target or binary returns non-zero so canonical delivery results record failure without changing the managed job terminal state.
- The missing-binary test isolates both `PATH` and `HOME` (and clears `HERMES_BIN`) so the helper's home-directory fallback cannot resolve a real Hermes installation on the test machine.
- Documentation shows MCP `run` with request-scoped target assignment inside `notify_command` and explains that `notification.state="armed"` proves persistence, not downstream delivery.

## Explicit Completion Conditions

- `skills/agent-exec/scripts/hermes-notify-hook` uses the current `hermes send` interface, validates required inputs, and its usage header documents the request-scoped `HERMES_NOTIFY_TARGET` assignment instead of managed-child `--env` routing or LLM provider/model options.
- Focused repository-local tests named with the `hermes_notify_hook` prefix in `tests/integration.rs` execute the tracked helper against a fake `hermes` binary and prove success and fail-closed paths.
- `skills/agent-exec/references/hermes.md` contains no fenced code block invoking `hermes notify` (prose that prohibits the obsolete command may remain) and documents the exact MCP invocation shape.
- Existing MCP armed-notification (`mcp_run_reports_only_persisted_notification_as_armed`) and notification-failure (`notify_failure_does_not_change_job_state`) tests remain green and are part of the rerun command.

## Out of Scope

- Automatic session or channel discovery.
- Slack API integration, credentials, retries, fanout, or delivery acknowledgement.
- Changes to the generic MCP response schema or command-sink dispatcher.
- Resuming an LLM session on completion.
