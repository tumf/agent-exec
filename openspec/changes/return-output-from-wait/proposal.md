---
change_type: implementation
priority: high
dependencies: []
references:
  - src/wait.rs
  - src/schema.rs
  - tests/integration.rs
  - openspec/specs/agent-exec/spec.md
---

# Return bounded output from wait

**Change Type**: implementation

## Problem / Context

`wait` currently returns the final state, exit code, and log byte totals, but not the command output. A caller that waits for completion must make a second `tail` call to obtain the result. This adds an avoidable agent round trip and conflicts with the product goal of collapsing command execution and observation into fewer interactions.

## Proposed Solution

Make every successful `wait` response include the current bounded `stdout` and `stderr` excerpts with the same range, total-byte, and encoding metadata used by `run` and `tail`.

When the job reaches a terminal state, the response contains its final bounded output. When the observation deadline expires while the job is still running, the response contains output available at that point and keeps terminal-only fields absent.

Reuse the existing bounded log-reading and output-shape implementation rather than introducing a second output contract. Complete logs remain persisted at their existing paths.

## Acceptance Criteria

- A terminal `wait` response includes bounded `stdout` and `stderr`, range metadata, total-byte counts, and encoding.
- A non-terminal deadline response includes output available at the deadline and does not invent an exit code.
- Large output remains bounded according to the existing inline-output ceiling while complete logs remain recoverable.
- CLI, HTTP `GET /wait/:id`, and MCP `wait` expose the same response behavior through their shared wait path.
- README, the one-minute demo, and integration guidance describe `wait` as returning completion output; `tail` remains available for later or repeated log retrieval.

## Explicit Completion Conditions

- `src/wait.rs` constructs wait responses using the shared bounded log reader and stable output metadata fields.
- `WaitData` serialization contains the output fields without changing the stdout single-JSON envelope.
- Integration tests fail if terminal output is absent, if deadline output is absent after the job emitted data, or if a large response exceeds the configured bound.
- HTTP and MCP tests verify the shared response includes command output after completion.
- `prek run -a` and `cflx openspec validate return-output-from-wait --strict --evidence warn` pass.

## Out of Scope

- Returning unbounded complete logs inline.
- Removing or changing `tail`.
- Changing wait duration, polling, cancellation, or job-lifetime semantics.
- Changing persisted stdout/stderr log files.
