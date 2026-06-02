---
change_type: implementation
priority: high
dependencies: []
references:
  - openspec/specs/agent-exec/spec.md
  - openspec/specs/agent-exec-run/spec.md
  - src/run.rs
  - src/start.rs
  - src/restart.rs
  - src/tail.rs
  - src/schema.rs
  - src/config.rs
  - tests/integration.rs
---

# Add Inline Output Compression

**Change Type**: implementation

## Problem/Context

`agent-exec` already reduces agent round trips by returning bounded inline observations from `run`, `start`, and `restart`, and bounded tail observations from `tail`. Large command outputs still consume excessive context even when only failures, summaries, or repeated-log aggregates are needed.

The requested behavior is to incorporate rtk-style output compression without invoking the external `rtk` command or changing the existing canonical raw observation fields. The feature must preserve the JSON-only stdout contract and the existing raw byte range semantics.

## Proposed Solution

Add built-in inline output compression for `run`, `start`, `restart`, and `tail`.

- Add `--compress <mode>` and `--rtk <mode>` as equivalent CLI options.
- Supported modes: `off`, `route`, `errors`, `tests`, `logs`, `git`, `json`, `summary`.
- Built-in default mode is `route`, so compression is on by default.
- Add `[compression].default` config support to override the default mode.
- Resolve mode by precedence: CLI `--compress`/`--rtk` > config `[compression].default` > built-in `route`.
- Preserve `stdout`, `stderr`, `stdout_range`, `stderr_range`, `stdout_total_bytes`, `stderr_total_bytes`, `encoding`, and log path fields as raw observation data.
- Add a `compression` response object when the resolved mode is not `off`.
- Do not call or require the external `rtk` binary.

## Acceptance Criteria

- `run`, `start`, `restart`, and `tail` accept `--compress <mode>` and `--rtk <mode>` for all supported modes.
- `--rtk <mode>` behaves exactly like `--compress <mode>`.
- `--compress` and `--rtk` with conflicting modes fail as a usage error with exit code `2`.
- With no CLI flag and no config, responses use compression mode `route` and include a `compression` object.
- `[compression].default = "off"` disables default compression without changing raw observation fields.
- CLI mode overrides config mode.
- Invalid CLI modes fail as usage errors; invalid config modes fail with a structured JSON error.
- Compression never replaces canonical raw `stdout`/`stderr` fields or changes their byte ranges.
- `off` mode omits the `compression` field.
- The implementation includes local integration tests that would fail for a no-op implementation.

## Explicit Completion Conditions

This proposal is complete when:

- `src/main.rs` exposes `--compress` and `--rtk` on `run`, `start`, `restart`, and `tail`.
- `src/config.rs` loads `[compression].default` and validates it against the supported mode set.
- Runtime option resolution follows CLI > config > built-in `route`.
- `src/schema.rs` includes a serializable compression response type used by relevant response payloads.
- Compression logic is implemented in repository source code without invoking `rtk`.
- `tests/integration.rs` covers default `route`, `off`, alias equivalence, CLI-over-config precedence, invalid config, conflicting flags, and at least one behavior-bearing compression mode such as `errors`, `tests`, or `logs`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` pass.

## Out of Scope

- Installing, shelling out to, or depending on the external `rtk` command.
- Hook-based command rewrite.
- Token analytics, telemetry, or SQLite history.
- Persisting compressed logs separately from existing raw logs.
- Replacing canonical raw observation fields with compressed text.
- Adding compression to `status`, `wait`, `kill`, `list`, or `serve` HTTP endpoints in this change.
