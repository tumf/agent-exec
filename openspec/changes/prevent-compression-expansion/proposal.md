---
change_type: implementation
priority: medium
dependencies: []
references:
  - openspec/specs/agent-exec-run/spec.md
  - src/compress.rs
  - src/run.rs
  - src/tail.rs
  - src/schema.rs
  - tests/integration.rs
---

# Prevent Compression Expansion

**Change Type**: implementation

## Problem/Context

Inline output compression is enabled by default through mode `route`. Compression is intended to reduce context use, but some command outputs are already compact or are structured in a way that a naive compressed view can become larger than the raw observed output. During local verification, a command whose output consisted of multiple `agent-exec` JSON responses caused `tail` to include a `compression.stdout` payload larger than the raw tail payload.

The `tail` target is still the command output. The issue is not that `tail` reads the wrong thing; the issue is that the optional compressed view can increase response size for some command-output shapes.

## Proposed Solution

Add a compression expansion guard shared by `run`, `start`, `restart`, and `tail` compression response generation.

When compression is enabled and a compressor produces output that is not smaller than the raw observed payload it is summarizing, `agent-exec` must not include the larger compressed text. Instead, it should return a small compression metadata object that indicates compression was skipped or not applied because it would expand the response.

Recommended response behavior:

- Keep canonical raw `stdout`/`stderr` fields unchanged.
- Keep `compression` present when resolved mode is not `off`.
- Set `compression.applied = false` when compressed payload would be larger than or equal to the raw payload.
- Set compressed `stdout`/`stderr` to empty strings or a bounded short diagnostic summary.
- Include a strategy or detected reason such as `expansion-guard` so callers can distinguish this from successful compression.
- Preserve existing `off` behavior: when mode is `off`, omit `compression` entirely.

## Acceptance Criteria

- `run`, `start`, `restart`, and `tail` do not include compressed text that is larger than or equal to the raw observed stream text it summarizes.
- When the guard triggers, the response remains JSON-only and includes a bounded `compression` object with `applied=false`.
- The guard does not modify canonical raw `stdout`/`stderr` fields or byte range fields.
- `off` mode still omits the `compression` object.
- Legitimately smaller compressed output, such as repeated-line log compression, still returns `applied=true` and the compact payload.
- Integration tests cover at least one command-output shape where the guard triggers, and at least one shape where compression still applies.

## Explicit Completion Conditions

This proposal is complete when:

- `src/compress.rs` or equivalent compression code compares compressed output size against the raw observed output size before returning compressed text.
- The fallback `compression` payload is bounded and cannot exceed the raw observed payload because of the guard path itself.
- `run`, `start`, `restart`, and `tail` use the guarded compressor output through their existing compression wiring.
- Integration tests verify guard behavior for a JSON/NDJSON-like command output that previously risked expansion.
- Existing compression tests for `errors`, `logs`, `json`, or `route` are adjusted only as needed to reflect `applied=false` on expansion.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Disabling default compression.
- Changing supported compression modes.
- Removing `tail` compression support.
- Replacing canonical raw output fields with compressed output.
- Adding token analytics or telemetry.
