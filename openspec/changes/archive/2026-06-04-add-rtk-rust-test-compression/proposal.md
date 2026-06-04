---
change_type: implementation
priority: high
dependencies:
  - add-rtk-compression-routing
references:
  - src/compress.rs
  - tests/integration.rs
  - rtk-ai/rtk/src/cmds/rust
  - rtk-ai/rtk/src/cmds/python/pytest_cmd.rs
  - rtk-ai/rtk/src/cmds/js/vitest_cmd.rs
  - rtk-ai/rtk/src/cmds/go/go_cmd.rs
---

# Add RTK-style Rust and Test Output Compression

**Change Type**: implementation

## Problem/Context

Test runners and Rust build commands produce large outputs where agents need failures, diagnostics, and summaries rather than every passing test or progress line.

## Proposed Solution

Add specialized compressors for `cargo test`, `cargo build`, `cargo check`, `cargo clippy`, and generic test outputs from common runners. These compressors focus on failures and compiler diagnostic blocks while aggregating passing/noisy output.

## Acceptance Criteria

- Passing test output is summarized by counts and does not include every passing test.
- Failing tests preserve failure names, assertion messages, and bounded stack/backtrace context.
- Rust compiler diagnostics preserve error/warning code, file:line, primary message, and relevant notes/help.
- Cargo progress lines are stripped or aggregated.
- Compression remains observation-only and expansion-guarded.

## Explicit Completion Conditions

Representative fixtures and integration commands prove compression for passing tests, failing tests, cargo diagnostics, and small-output guard cases.

## Dependencies

Requires `add-rtk-compression-routing`.

## Out of Scope

- JS/Python/Go lint/build-specific parsing beyond generic test patterns.
