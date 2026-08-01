---
change_type: implementation
priority: high
dependencies: []
references:
  - Cargo.toml
  - src/lib.rs
  - src/main.rs
  - src/run.rs
  - src/status.rs
  - src/tail.rs
  - src/kill.rs
  - src/list.rs
  - tests/integration.rs
verifications:
  - id: embedded-api-local
    requirement: A separate Rust consumer manages detached agent-exec jobs through typed library calls without invoking the agent-exec CLI
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: tests/integration.rs
    evidence: integration test output proving typed run/status/tail/list/kill and post-consumer supervisor continuity
    rerun: cargo test --test embedded_consumer && cargo test --test integration
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Add an Embedded Managed-Job API

**Change Type**: implementation

## Problem / Context

The crate exposes command modules, but it does not yet provide a coherent embedding contract. `status_response`, `tail_response`, and `kill_response` return typed responses, while `list::execute` prints directly to stdout. More importantly, `run_response` launches the detached supervisor by invoking `std::env::current_exe()` with the private `_supervise` subcommand. When another Rust binary links the crate, `current_exe()` identifies that consumer binary; unless the consumer understands agent-exec's hidden CLI arguments, every embedded job launch fails.

Consumers such as `beads-runner` therefore spawn the installed `agent-exec` CLI for every run, status, tail, list, and kill operation and reparse JSON. This adds executable discovery, subprocess, stdout framing, and version-skew work even though the typed implementation is already linked.

## Proposed Solution

Provide a small public embedded client that returns typed data for `run`, `status`, `tail`, `list`, and `kill` without printing JSON or invoking the public agent-exec CLI. Preserve the detached supervisor process: embedded launch SHALL re-execute an explicit supervisor executable and the consumer SHALL delegate the reserved supervisor invocation to a public agent-exec entrypoint before normal consumer argument parsing.

The default embedded constructor SHALL use the current consumer executable as the supervisor executable. A consumer opts in by calling the delegation entrypoint at process startup; when no reserved invocation is present, control returns immediately to the consumer. An explicit executable override SHALL exist for tests and advanced packaging. The normal `agent-exec` CLI SHALL use the same typed client and delegation path, keeping one implementation of job semantics.

## Acceptance Criteria

- A separate Rust consumer can construct one client with an explicit jobs root and call typed `run`, `status`, `tail`, `list`, and `kill` operations without spawning the public `agent-exec` CLI or parsing JSON.
- Embedded `run` preserves the existing default bounded inline observation behavior and allows an explicit no-wait launch.
- A launched job remains managed after the original consumer process exits because supervision remains in a detached re-executed process.
- The consumer's startup delegation claims only an exact reserved marker at `argv[1]`; invocations without it remain untouched, while claimed invocations with malformed generated arguments fail closed before consumer argument handling.
- Missing startup delegation, malformed delegated arguments, or an unusable supervisor executable causes failure within a fixed five-second startup acknowledgement deadline, records terminal `failed` without an intermediate `running` transition, and does not launch a workload or emit a completion notification.
- Typed status, tail, list, and kill preserve current job lookup, tag filtering, byte totals/ranges, state, exit-code, and TERM semantics.
- `list` gains a non-printing typed response path; library calls never write command JSON to consumer stdout or diagnostics to consumer stdout.
- The public CLI's JSON envelopes, exit codes, help, defaults, jobstore layout, logs, notifications, masking, timeout behavior, process-tree handling, and Windows Job Object behavior remain compatible.
- Errors exposed by the embedded client are structured enough for consumers to distinguish missing jobs, ambiguous IDs, invalid input/state, launch failure, and I/O/internal failure without parsing message text.

## Explicit Completion Conditions

- `src/lib.rs` exports the embedded client, typed request/result/error surface, and supervisor startup delegation entrypoint.
- `src/run.rs` no longer unconditionally derives supervisor execution from an undelegated `current_exe()` assumption; launch uses the client-selected executable and rollback-safe state transitions.
- `src/list.rs` exposes a typed non-printing operation, and CLI printing wraps typed results rather than duplicating enumeration.
- A fixture consumer binary or integration-test helper links the crate, delegates reserved startup, starts a real detached job, exits its launching process, and verifies the job through typed status/tail/list/kill calls.
- A negative fixture omits delegation and proves launch fails instead of leaving a permanently running or unrecoverable record.
- `cargo test --test embedded_consumer`, existing CLI integration tests, formatting, clippy, and default tests pass.

## Out of Scope

- Running the supervisor as an in-process thread or tying job lifetime to the embedding process.
- Removing the standalone CLI, MCP server, HTTP server, or JSON contract.
- Providing async APIs; the current jobstore and command surface remain synchronous.
- Embedding arbitrary consumer callbacks inside the detached supervisor.
- Changing persisted job schemas, IDs, output files, notification contracts, or process-tree semantics.
- Modifying `beads-runner` in this repository.
