---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/compress/route.rs
  - src/compress/generic.rs
  - src/compress/util.rs
  - openspec/specs/agent-exec-run/spec.md
---

# Refactor Compression Routing

**Change Type**: implementation

## Problem / Context

Compression routing is spread across large match expressions and many loosely connected helper functions in `src/compress/route.rs`, `src/compress/generic.rs`, and `src/compress/util.rs`. The code handles command-family detection, output-shape heuristics, summary strategies, and expansion guarding. As more command families have been added, the routing and summarization responsibilities have become harder to review and extend without accidentally changing classification priority or compression contracts.

## Proposed Solution

Split compression logic into clearer responsibilities while preserving existing behavior: command classification, output-shape classification, family-specific summarizers, and safety guards. Keep `CompressionData`, detected-kind stable strings, supported modes, route priority, raw observation fields, and expansion-guard behavior unchanged.

## Acceptance Criteria

- Route classification priority remains unchanged for command-family routes, JSON output, repeated logs, tests, psql tables, errors, and summary fallback.
- Family-specific summarizers remain behaviorally equivalent for Git, tests/errors, language diagnostics, tables, cloud/container/HTTP, system/search/file/env, and JSON summaries.
- Compression output still applies `fallback_if_empty` and expansion guard semantics exactly as before.
- Existing compression tests continue to pass, and new focused tests cover at least one refactored boundary between classification and summarization.
- Public response fields and `compression.detected_kind` stable strings remain unchanged.

## Explicit Completion Conditions

- Compression code has explicit module or helper boundaries for routing/classification versus summarization, reducing the need to edit one large match for unrelated families.
- Tests verify representative routed outputs before and after the refactor for at least Git, repeated logs, JSON, search, and language/test diagnostics.
- Verification passes with `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

## Out of Scope

- Adding new compression modes or supported command families.
- Changing compression output text formats intentionally.
- Replacing built-in compression with external `rtk` invocation.
- Changing canonical raw stdout/stderr observation fields.
