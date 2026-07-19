# AI agent integrations

`agent-exec mcp` exposes five managed-job tools over stdio: `run`, `status`, `tail`, `wait`, and `kill`.

## Prerequisites

Install `agent-exec`, then resolve its absolute path. MCP clients may not inherit your interactive shell's `PATH`, so the examples use `AGENT_EXEC` explicitly.

```bash
AGENT_EXEC="$(command -v agent-exec)"
test -x "$AGENT_EXEC"
agent-exec --version
```

After configuration, ask the agent to start a command with `run`, retain the returned `job_id`, and use `wait` for completion state and bounded output. Use `tail` later for repeated log retrieval. An MCP request deadline or disconnected client does not stop the managed job. Use `kill` only for explicit cancellation.

## Claude Code

Add the stdio server to the current project's local configuration. Use `--scope user` instead when every project should see it.

```bash
claude mcp add --scope local agent-exec -- "$AGENT_EXEC" mcp
claude mcp get agent-exec
```

Expected result: `Status: Connected` and the five managed-job tools are available as `mcp__agent-exec__*`.

## Codex CLI

Codex stores this server in its global MCP configuration.

```bash
codex mcp add agent-exec -- "$AGENT_EXEC" mcp
codex mcp get agent-exec
```

Expected result: `enabled: true`, `transport: stdio`, and `args: mcp`.

## OpenCode

OpenCode reads project configuration from `opencode.json`. Replace the command with the absolute path printed by `command -v agent-exec`.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "agent-exec": {
      "type": "local",
      "command": ["/absolute/path/to/agent-exec", "mcp"],
      "enabled": true
    }
  }
}
```

Verify the resolved server:

```bash
opencode mcp list
```

Expected result: `agent-exec` is connected.

## Hermes Agent

Add the stdio server through Hermes. The current Hermes CLI asks before overwriting an existing entry.

```bash
hermes mcp add agent-exec --command "$AGENT_EXEC" --args mcp
hermes mcp test agent-exec
```

Expected result: the connection succeeds and Hermes discovers five tools.

Equivalent `~/.hermes/config.yaml` configuration:

```yaml
mcp_servers:
  agent-exec:
    command: /absolute/path/to/agent-exec
    args: ["mcp"]
    env:
      AGENT_EXEC_MCP_DEFAULT_UNTIL_SECONDS: "10"
      AGENT_EXEC_MCP_MAX_UNTIL_SECONDS: "55"
```

Keep `AGENT_EXEC_MCP_MAX_UNTIL_SECONDS` below the MCP client's request timeout so the response has time to return.

## CLI fallback

If an agent cannot use MCP, install the embedded skill and use the same lifecycle through CLI commands.

```bash
agent-exec install-skills
# Claude Code location instead:
agent-exec install-skills --claude
```

Without `--global`, the skill is installed under `.agents/skills` in the current directory, not under `$HOME`. Use `--global` for the global agent-skills directory. The success JSON reports the resolved installation `path`; verify that path before configuring the agent.

The agent can then call `agent-exec run`, retain `job_id`, use `wait` for completion output, and use `tail` for later or repeated log retrieval.
