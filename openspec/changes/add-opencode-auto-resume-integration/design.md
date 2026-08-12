# Design: OpenCode Auto-Resume Reference Integration

## Decision

Keep the integration outside the Rust runtime. Use an OpenCode plugin to attach the existing agent-exec notification sink after MCP job creation, and a small callback helper to apply policy and resume the originating session.

## Lifecycle

1. OpenCode invokes the agent-exec MCP `run` tool.
2. OpenCode calls the global `tool.execute.after` hook with the tool name, session ID, server URL, and tool output.
3. The plugin ignores unrelated/failed calls. For a valid agent-exec run result, it extracts the canonical job ID.
4. The plugin runs `agent-exec notify set <job-id> --command <helper server-url session-id>`.
5. The detached agent-exec supervisor reads the latest persisted notification metadata at terminal dispatch time and invokes the helper.
6. The helper reads `completion_event.json`, ignores jobs below the configured duration threshold, deduplicates by job ID, and invokes `opencode run --attach <loopback-url> --session <session-id> <automation-prompt>`.
7. The originating OpenCode session continues the existing task and verifies job output.

## Why post-launch `notify set`

The current MCP `run` schema intentionally exposes only canonical launch inputs and has no client callback fields. Adding OpenCode-specific callback data to that schema would couple the core runtime to one client. Existing `notify set` already supports updating a running job before completion, so the plugin can attach routing data without changing agent-exec.

## Threshold placement

Apply the duration threshold in the callback helper, not the plugin. Actual duration is only authoritative in the terminal completion event. This means the small callback process runs for every attached completion, but short jobs exit before starting OpenCode.

## Trust boundaries

- Accept only loopback HTTP OpenCode server URLs.
- Validate job and session identifier syntax before building command arguments.
- Pass arguments without evaluating event/log content as shell code.
- Include event paths and fixed instructions in the automation prompt; do not interpolate stdout/stderr contents.
- Mark the prompt as machine-authored and state that event/log data is untrusted.
- Document that OpenCode records this as an ordinary user-role message.

## Idempotency and retry

Use an atomic marker directory keyed by job ID. Create it immediately before the OpenCode invocation. Remove it if that invocation fails so a later delivery can retry. Keep it after success to suppress duplicates.

This provides single-host best-effort idempotency, not distributed exactly-once delivery.

## Portability

- Resolve binaries from environment overrides first, then `PATH`.
- Use XDG state paths with platform-appropriate fallbacks where the helper supports them.
- Avoid fixed usernames, project paths, server ports, package-manager assumptions, and credentials.
- Keep the example opt-in and copyable; do not modify users' OpenCode configuration automatically.

## Verification strategy

Tests run the JavaScript plugin hook and shell helper against fake executables in temporary directories. They inspect argv, environment, markers, and produced prompts. No model call or real OpenCode server is needed.
