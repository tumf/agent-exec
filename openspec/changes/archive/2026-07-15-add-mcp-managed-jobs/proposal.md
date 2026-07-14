---
change_type: hybrid
priority: high
dependencies: []
references:
  - https://github.com/tumf/agent-exec/issues/7
  - src/main.rs
  - src/run.rs
  - src/status.rs
  - src/tail.rs
  - src/wait.rs
  - src/kill.rs
  - src/serve.rs
  - skills/agent-exec/SKILL.md
  - tests/integration.rs
---

# Add MCP Managed Jobs

**Change Type**: hybrid

## Problem/Context

`agent-exec run` starts a detached `_supervise` process so a job outlives the launching CLI process. Hermes and similar agents may instead launch commands through a terminal implementation that owns and later waits for or terminates its whole process tree. Launching `agent-exec` through that terminal can interfere with the detached job lifecycle.

The repository already has `agent-exec serve`, but it is not a suitable MCP adapter: `POST /exec` duplicates parts of the run path, `GET /wait/:id` waits indefinitely, and `GET /tail/:id` uses fixed observation bounds. The MCP surface must reuse shared job-operation behavior rather than adding another divergent execution path.

## Proposed Solution

Add `agent-exec mcp [--root <PATH>]`, a stdio MCP server exposing five tools: `run`, `status`, `tail`, bounded `wait`, and `kill`.

- Use an in-process Rust adapter over shared job-operation functions. Do not make MCP depend on a running HTTP `serve` instance.
- Refactor existing CLI operation modules so they construct existing `Response<T>` envelopes before a thin CLI printing boundary; MCP returns those envelopes as structured tool output.
- Make `run` create jobs through the same persisted metadata, supervisor, observation, and JSON contract used by CLI `run`.
- Make MCP `wait` bounded only: default 30 seconds and optional `until` in seconds. Do not expose unbounded `forever` waiting through the MCP tool.
- Keep MCP protocol frames exclusively on stdout; diagnostics and tracing remain on stderr.
- Keep the existing CLI and HTTP `serve` public contracts unchanged, including loopback and authentication policy for `serve`.
- Update the embedded agent skill with MCP-first guidance for Hermes-like clients, CLI fallback, bounded observation, and explicit-user-request-only cancellation.

The scopes are coupled: the MCP adapter cannot truthfully preserve the existing response and lifecycle contracts without the shared-operation refactor, integration tests, and skill guidance landing together.

## Acceptance Criteria

- `agent-exec mcp --help` exposes a stdio MCP server and optional common jobs root without changing existing subcommand behavior.
- MCP initializes successfully and advertises exactly `run`, `status`, `tail`, `wait`, and `kill` as managed-job tools.
- MCP `run(command, cwd?, env?, timeout?, until?)` accepts a non-empty argv array, starts a persisted job through the canonical run path, and returns the existing `type="run"` envelope containing job ID, state, inline stdout/stderr, byte ranges, totals, and log paths.
- MCP `status(job_id)`, `tail(job_id, lines?, max_bytes?)`, `wait(job_id, until?)`, and `kill(job_id)` return their existing response envelopes and honor job-ID resolution rules.
- MCP `wait` defaults to a 30-second maximum and returns a non-terminal state without stopping the job when its deadline is reached.
- Closing the MCP client transport after `run` does not kill or wait indefinitely for the managed job; a separate CLI or later MCP connection can observe that job.
- `kill` sends a signal only when its tool is explicitly called; disconnecting, timed-out observation, and tool errors never imply cancellation.
- stdout contains valid MCP JSON-RPC frames only; tracing and diagnostics do not corrupt the stdio transport.
- Existing CLI and HTTP `serve` behavior remains covered and unchanged; `serve` retains its loopback-default and non-loopback authentication safeguards.
- The embedded `skills/agent-exec/SKILL.md` instructs MCP-capable clients to use MCP for uncertain or long-running work, retain job IDs, observe with `status`/`tail`/bounded `wait`, use `kill` only on explicit user cancellation, and fall back to CLI where MCP is not configured.

## Explicit Completion Conditions

This proposal is complete when:

- `src/main.rs` dispatches `agent-exec mcp` without emitting non-protocol stdout.
- A dedicated MCP module registers the five tools on an stdio transport and maps valid/invalid calls to stable existing envelopes or MCP tool errors without exposing process control outside the defined tools.
- Shared source functions construct `Response<RunData>`, `Response<StatusData>`, `Response<TailData>`, `Response<WaitData>`, and `Response<KillData>` without requiring stdout printing, while existing CLI commands still print exactly one JSON envelope.
- MCP run uses the canonical job creation/supervisor path, and MCP observation functions use caller-provided/default bounds instead of fixed HTTP-only values.
- Integration tests execute MCP initialize, tool discovery, successful job lifecycle observation, client-disconnect persistence, bounded wait, explicit kill, invalid tool input/job ID behavior, and stdout protocol purity.
- Existing REST and CLI tests continue to pass without behavior changes.
- The embedded skill documents MCP configuration and fallback operational rules.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`, `cflx openspec validate add-mcp-managed-jobs --strict --evidence warn`, and `cflx openspec validate add-mcp-managed-jobs --archive-gate` pass.

## Out of Scope

- General-purpose arbitrary-shell MCP tooling or replacement of `mcp-shell-server`.
- Network-accessible MCP transport, HTTP-to-MCP bridging, or changing `serve` bind/auth policy.
- MCP support for every CLI definition option; initial `run` supports only command, cwd, env, timeout, and until.
- MCP tools for create, start, restart, list, delete, GC, notifications, tags, stdin materialization, masking, compression configuration, or schema introspection.
- Automatic cancellation on MCP disconnect, wait deadline, or tool failure.
