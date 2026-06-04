---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/run.rs
  - openspec/specs/agent-exec/spec.md
---

# Refactor CLI Option Groups

**Change Type**: implementation

## Problem / Context

`src/main.rs` defines a large `Command` enum where `Create`, `Run`, `Start`, `Restart`, and hidden `_supervise` repeat related option families such as job root, auto-GC, inline observation, compression, environment, stdin, notifications, tags, and shell wrapper settings. The duplication increases the chance that definition-time options drift between `run` and `create`, even though the canonical spec requires shared persisted definition inputs.

## Proposed Solution

Introduce small internal option group types and conversion helpers that preserve the existing clap surface and JSON contract while reducing duplicated execution wiring. The refactor should keep user-facing flags, aliases, defaults, conflicts, completions, and response behavior unchanged.

## Acceptance Criteria

- `agent-exec create` and `agent-exec run` continue to expose equivalent persisted definition-time options where required by the spec.
- Inline observation options used by `run`, `start`, and `restart` are represented through a shared internal structure before command execution.
- Auto-GC options used by `run`, `start`, and `restart` are represented through a shared internal structure before command execution.
- Existing clap behavior, usage errors, dynamic completions, and stdout response schema remain unchanged.
- Existing integration tests continue to pass without changing test intent.

## Explicit Completion Conditions

- `src/main.rs` contains shared internal option structures or equivalent helpers for at least definition-time, auto-GC, and inline-observation option families.
- Execution dispatch still maps all existing command variants to the same module-level execute functions and preserves all existing argument defaults and conflicts.
- Verification passes with `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.
- At least one integration or unit verification checks that `run` and `create` still persist the same representative definition metadata for tags, notification settings, env settings, and stdin settings.

## Out of Scope

- Adding, removing, or renaming public CLI flags.
- Changing stdout JSON schemas, exit codes, or persisted `meta.json` / `state.json` shapes.
- Rewriting command execution, supervisor lifecycle, or auto-GC behavior.
