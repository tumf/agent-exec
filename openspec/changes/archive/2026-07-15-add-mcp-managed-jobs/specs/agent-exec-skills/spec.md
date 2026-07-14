## ADDED Requirements

### Requirement: MCP-first managed job guidance

Embedded `skills/agent-exec/SKILL.md` must document MCP-first operation for MCP-capable agent clients such as Hermes (MUST). The skill must instruct such clients to start uncertain-duration, long-running, or high-output commands through agent-exec MCP `run` rather than a terminal process manager (MUST), retain returned job IDs, and observe jobs with MCP `status`, `tail`, or bounded `wait` (MUST).

The skill must state that MCP `kill` is reserved for an explicit user cancellation request (MUST). MCP client disconnect, bounded wait expiry, missing output, or a transition to other work must not be treated as cancellation authorization (MUST NOT).

#### Scenario: installed skill documents bounded MCP lifecycle

**Given**: `agent-exec install-skills` installs the embedded skill
**When**: the installed `SKILL.md` is read
**Then**: it describes MCP `run`, job-ID retention, `status`/`tail`/bounded `wait`, and explicit-user-request-only `kill`

### Requirement: MCP fallback and Hermes configuration guidance

The embedded skill must include an MCP configuration example for Hermes Native MCP that launches `agent-exec mcp` (MUST). The skill must document CLI `agent-exec run -- <command>` as fallback when MCP is unavailable (MUST). It must preserve the exception for clearly short, synchronous, safe inline shell commands (MAY).

#### Scenario: installed skill retains CLI fallback

**Given**: `agent-exec install-skills` installs the embedded skill
**When**: the installed `SKILL.md` is read
**Then**: it includes a Hermes Native MCP configuration example
**And**: it includes CLI fallback guidance for MCP-unconfigured environments
**And**: it does not instruct a client to kill a job merely because an observation deadline elapsed
