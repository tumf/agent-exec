---
change_type: implementation
priority: high
dependencies: []
references:
  - src/compress/route.rs
  - src/compress/generic.rs
  - src/compress/util.rs
  - tests/integration.rs
  - openspec/specs/agent-exec-run/spec.md
---

# Fix RTK Log Route Priority

**Change Type**: implementation

## Problem/Context

A live `agent-exec --rtk route` demo showed that timestamp-varied repeated `ERROR` logs were classified as `errors` instead of `logs`. Because the `errors` compressor keeps error-bearing lines, the output remained almost as large as raw output (`3200` bytes raw vs `3199` bytes compressed). Explicit `--rtk logs` on the same output used `dedupe-normalized-log-lines` and reduced the output substantially.

The current route logic already prioritizes adjacent exact repeated lines before generic error detection, but it does not recognize timestamp-normalized repeated log shapes before `looks_like_error_output` wins.

## Proposed Solution

Update route compression so repeated or timestamp-normalized log-like output is classified as `logs` before generic error classification. Keep single non-repeated error/panic/traceback output classified as `errors`.

The intended route priority for output-shape fallback is:

1. command-family route
2. JSON/NDJSON shape
3. repeated or timestamp-normalized log shape
4. test output
5. database/table shape where applicable
6. generic error/panic/traceback
7. summary

Implementation may choose a slightly different order for table/test if existing behavior requires it, but repeated/normalized logs must beat generic errors.

## Acceptance Criteria

- `agent-exec run --rtk route -- <timestamped repeated ERROR log command>` reports `compression.detected_kind = "logs"`.
- The same response includes `dedupe-normalized-log-lines` in `compression.strategy`.
- The compressed stdout is substantially smaller than raw stdout for timestamp-varied repeated ERROR logs.
- A single non-repeated error output still reports `compression.detected_kind = "errors"`.
- Existing exact adjacent repeated log routing remains `logs`.
- Existing raw stdout/stderr and range fields remain unchanged.

## Explicit Completion Conditions

This change is complete when:

- `src/compress/route.rs` recognizes timestamp-normalized repeated log output before generic error output.
- Unit or integration coverage demonstrates both the fixed repeated-ERROR-log path and the preserved single-error path.
- The demo command used to expose the issue produces `detected_kind="logs"` and a meaningful byte reduction.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and relevant compression tests pass.

## Out of Scope

- Replacing the logs compressor itself.
- Changing explicit `--rtk errors` behavior.
- Changing raw output fields or log persistence.
- Introducing command rewriting or external RTK dependency.
