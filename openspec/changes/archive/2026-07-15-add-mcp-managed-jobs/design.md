# Design: MCP Managed Jobs

## Requested Artifact

Both: an implementation-driving proposal and tracked specification deltas.

## Request Normalization

### User-facing outcomes

- MCP-capable agent clients launch uncertain or long-running work without routing the launch through their terminal process manager.
- A returned job ID remains usable for later status, log-tail, bounded completion observation, and explicit cancellation.
- MCP transport lifecycle does not become managed-job lifecycle.
- Existing CLI and REST users retain their current contracts.

### Repository areas likely requiring change

- `Cargo.toml`: add the official Rust MCP SDK and only its required features.
- `src/main.rs`: add the `mcp` subcommand and dispatch.
- `src/mcp.rs`: stdio MCP transport, tool schemas, and thin adapter.
- `src/run.rs`, `src/status.rs`, `src/tail.rs`, `src/wait.rs`, `src/kill.rs`: returnable response constructors shared by CLI and MCP.
- `src/serve.rs`: reuse shared constructors where this reduces its current contract drift without changing HTTP behavior.
- `tests/integration.rs` and test support: stdio JSON-RPC lifecycle coverage.
- `skills/agent-exec/SKILL.md`: MCP-first operational guidance.

## Architecture

### Transport boundary

`agent-exec mcp` owns stdin and stdout for one MCP stdio session. Only MCP JSON-RPC messages write to stdout. `tracing_subscriber` remains configured for stderr, and no tool implementation calls `Response::print()`.

The MCP server is not a job supervisor and does not spawn a long-lived process tree of its own. `run` invokes the existing supervisor-launch mechanism, which detaches `_supervise` before the MCP tool result is emitted. Therefore, MCP session closure must only close the protocol transport.

### Shared operation boundary

The current command modules combine response construction and stdout printing. Extract narrow `*_response` or `*_data` functions that return the existing typed `Response<T>` values:

- `run`: canonical persisted job creation, supervisor launch, and inline observation.
- `status`: JobDir state lookup.
- `tail`: bounded tail lookup with caller-selected lines and bytes.
- `wait`: bounded polling with the existing terminal/non-terminal semantics.
- `kill`: existing explicit signal behavior.

CLI adapters preserve their current behavior by printing returned envelopes at their outer boundary. HTTP handlers and MCP tools serialize the same return values. This is deliberately not a new generic job service abstraction: five shared constructors are enough to stop response/lifecycle divergence.

### Tool contract

| Tool | Parameters | Defaults and limits | Result |
| --- | --- | --- | --- |
| `run` | `command: string[]`, `cwd?: string`, `env?: object<string,string>`, `timeout?: number`, `until?: integer` | command non-empty; timeout and until are seconds; until defaults 10 | existing `run` envelope |
| `status` | `job_id: string` | none | existing `status` envelope |
| `tail` | `job_id: string`, `lines?: integer`, `max_bytes?: integer` | 50 lines; 65536 bytes | existing `tail` envelope |
| `wait` | `job_id: string`, `until?: integer` | 30 seconds; no forever option | existing `wait` envelope |
| `kill` | `job_id: string` | TERM; preserve current post-signal observation | existing `kill` envelope |

`env` converts deterministically to `KEY=VALUE` inputs. Validate env keys and values at the MCP trust boundary before passing them into canonical runtime validation. The initial surface intentionally excludes sensitive/persisted definition controls such as stdin, mask, notification, tags, and environment files.

### Errors

Malformed tool arguments produce MCP tool errors with a stable, actionable message and no spawned job. Canonical domain failures such as `job_not_found`, `ambiguous_job_id`, and `invalid_state` are returned as the repository's existing `ok=false` JSON envelopes so agent clients can consume stable `error.code` values. Tool errors, deadlines, and transport closure do not invoke `kill`.

### HTTP compatibility

MCP does not call `agent-exec serve` and does not loosen its network policy. The proposal may reuse the same response constructors from `serve`, but it must preserve endpoint paths, request/response compatibility, loopback defaults, and existing non-loopback token safeguards. Bounded MCP `wait` is intentionally distinct from the current HTTP endpoint contract and does not silently alter it.

## Verification Strategy

Integration coverage is primary because the feature crosses a CLI process, MCP stdio framing, detached supervisor lifecycle, and persisted state.

- `unit`: parameter conversion and validation for `run.env`, empty commands, defaults, and rejected non-finite/negative duration values.
- `integration`: spawn the compiled `agent-exec mcp` binary, send initialize/tools-list/call-tool JSON-RPC messages, and parse stdout frames without accepting extra text.
- `integration`: run a sleep-based managed job with a bounded launch observation, close the first MCP connection, then use CLI or a second MCP connection to assert the job remains observable and later terminal.
- `integration`: bounded `wait` returns a live state with no exit code and the workload remains live; a later wait/tail verifies completion.
- `integration`: `kill` only changes a running job when explicitly called; a disconnected session or a bounded wait does not change it.
- `integration`: invalid args and unknown job IDs produce protocol-safe errors/envelopes and no extra job directory.
- `manual`: verify the documented Hermes Native MCP configuration launches the compiled binary with the configured root; this is intentional because Hermes configuration ownership lives outside this repository.

## Trade-offs

The official MCP SDK adds a protocol dependency rather than hand-writing JSON-RPC framing. This is the smallest reliable route for initialization negotiation, tool schemas, and stdio lifecycle behavior.

MCP intentionally exposes fewer `run` options than CLI. Matching every CLI definition option would create a broad remote execution configuration surface without being needed to solve terminal-tree interference. Add options only when a concrete MCP client workflow requires them and tests define their persistence/security behavior.
