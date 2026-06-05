---
change_type: implementation
priority: medium
dependencies: []
references:
  - tests/integration.rs
  - tests/serve_integration.rs
  - openspec/specs/agent-exec-test-harness/spec.md
---

# Refactor Integration Test Support

**Change Type**: implementation

## Problem / Context

`tests/integration.rs` has grown into a large test file with many helpers and thousands of lines of command-level contract checks. Although a basic `TestHarness` exists, command execution, raw output assertions, usage-error assertions, global-root/subcommand-root variants, stdin handling, and JSON envelope checks remain localized in one large file. This makes future contract tests harder to add and increases the risk of inconsistent assertion quality.

## Proposed Solution

Extract reusable integration-test support into focused helper modules while preserving all existing test cases and their intent. The support layer should make isolated roots, binary invocation, stdout JSON parsing, usage-error checks, stdin injection, and envelope assertions reusable across integration suites.

## Acceptance Criteria

- Integration tests still create isolated temporary roots and set `AGENT_EXEC_ROOT` consistently.
- JSON-only stdout assertions, usage-error assertions, raw command execution, stdin handling, and global/subcommand root invocation helpers are reusable without duplicating process-spawn logic.
- Existing tests in `tests/integration.rs` and `tests/serve_integration.rs` keep their behavior and continue to pass.
- The refactor does not remove contract coverage for stdout JSON envelopes, exit codes, persisted metadata, lifecycle commands, compression, notifications, tags, or server behavior.

## Explicit Completion Conditions

- A shared test-support module or equivalent helper organization exists under `tests/` and is used by multiple test sections or files.
- Existing test names and assertion intent remain recognizable after the refactor.
- Verification passes with `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.
- At least one test path using stdin, one path expecting usage error, and one path using isolated root execution are covered through the shared support layer.

## Out of Scope

- Changing product behavior or CLI contracts.
- Deleting tests solely to reduce file size.
- Converting integration tests into unit tests unless behavior coverage is preserved.
- Marking tests `heavy` unless a test demonstrably exceeds the project performance policy and cannot be optimized.
