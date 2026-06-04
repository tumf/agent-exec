---
change_type: implementation
priority: high
dependencies: []
references:
  - src/compress.rs
  - src/schema.rs
  - src/run.rs
  - src/start.rs
  - src/restart.rs
  - src/tail.rs
  - tests/integration.rs
  - openspec/specs/agent-exec-run/spec.md
  - rtk-ai/rtk docs/contributing/ARCHITECTURE.md
---

# Add RTK-style Compression Routing Foundation

**Change Type**: implementation

## Problem/Context

`agent-exec` currently exposes built-in compression through `--compress` and `--rtk`, but the implementation is a small set of broad heuristics in `src/compress.rs`. The original RTK model is command-family aware: it routes output through specialized filters for git, test runners, build tools, logs, JSON, tables, and cloud/container commands.

Before adding RTK-equivalent filters across many command families, `agent-exec` needs a routing and module foundation that preserves its existing observation contract: raw `stdout` / `stderr`, byte ranges, total byte counts, and log paths remain canonical, while `compression.stdout` / `compression.stderr` provide a compact view.

## Proposed Solution

Split the compression implementation into a modular command-routing architecture and extend the compression metadata vocabulary so specialized proposals can plug in safely.

The foundation will:

- Replace the monolithic compression logic with a module tree such as `src/compress/`.
- Introduce typed command-family detection from `CompressionInput.command` and output shape.
- Preserve existing modes: `off|route|errors|tests|logs|git|json|summary`.
- Preserve existing `--rtk` alias and config precedence semantics.
- Emit stable, specific `detected_kind` values for routed families.
- Keep expansion guard behavior globally enforced.
- Provide shared utilities for truncation, line grouping, table parsing, JSON shape extraction, diagnostic blocks, and recovery-oriented summaries.

## Acceptance Criteria

- `agent-exec run/start/restart/tail` continue returning canonical raw observation fields unchanged.
- `--compress` and `--rtk` continue resolving effective modes with the existing precedence and conflict behavior.
- `route` mode classifies command families without invoking external `rtk` or rewriting the command.
- Compression responses can report more specific `detected_kind` values while preserving schema compatibility.
- Expansion guard remains applied to every specialized compressor.
- Existing compression integration tests continue to pass.
- New route-classification tests prove that representative commands map to the expected command family.

## Explicit Completion Conditions

This change is complete when:

- Compression code is modularized without changing public raw observation fields.
- `CompressionInput.command` is used by a dedicated route classifier with unit coverage.
- Existing compression behavior remains covered by `cargo test --test integration compression` or equivalent test filters.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.
- Strict OpenSpec validation for this change passes.

## Dependencies

None. This proposal is the root dependency for the remaining RTK-equivalent compression proposals.

## Out of Scope

- Implementing all specialized command-family compressors.
- Changing the raw stdout/stderr contract.
- Rewriting commands before execution.
- Calling or depending on the external `rtk` binary.
