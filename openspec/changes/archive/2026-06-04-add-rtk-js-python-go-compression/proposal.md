---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/compress.rs
  - tests/integration.rs
  - rtk-ai/rtk/src/cmds/js
  - rtk-ai/rtk/src/cmds/python
  - rtk-ai/rtk/src/cmds/go
---

# Add RTK-style JS Python Go Compression

**Change Type**: implementation

## Problem/Context

JS/TS, Python, and Go tooling produce structured or semi-structured output where RTK achieves large savings through rule grouping, JSON/NDJSON parsing, and state-machine summaries.

## Proposed Solution

Add specialized observed-output compressors for TypeScript, JS linters/builds/test runners, Python ruff/mypy/pytest/pip, and Go build/test/vet/golangci-lint output.

## Acceptance Criteria

- TypeScript and lint outputs are grouped by file, rule/code, and severity.
- JS/Python/Go test outputs preserve failures and summarize passes.
- JSON/NDJSON outputs from tools are parsed when already present in observed output.
- Package/dependency list outputs are compacted into bounded package summaries.
- Compression remains observation-only and expansion-guarded.

## Explicit Completion Conditions

Fixture-backed tests prove each language family reduces representative large output while preserving diagnostic identity and failure context.

## Dependencies

Requires `add-rtk-compression-routing`. It can reuse failure-focused helpers from `add-rtk-rust-test-compression`.

## Out of Scope

- Injecting `--json` flags into commands.
- Container/cloud/GitHub CLI compression.
