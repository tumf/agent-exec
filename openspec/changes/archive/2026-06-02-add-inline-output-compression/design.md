# Design: Inline Output Compression

## Overview

The feature adds a compact, rtk-style view of observed command output while preserving `agent-exec`'s existing raw observation contract. `agent-exec` remains a job execution and observation CLI, not a command proxy. Compression is therefore attached to observation responses rather than replacing stdout/stderr logs or rewriting commands.

## Requested Artifact

Implementation.

## Request Normalization

User-facing outcomes:

- Compression is enabled by default through mode `route`.
- Users can override mode per command with `--compress <mode>` or alias `--rtk <mode>`.
- Users can configure the default with `[compression].default`.
- The external `rtk` command is never used.
- Existing raw JSON observation fields remain trustworthy and byte-range based.

Repository areas likely requiring change:

- `src/main.rs` for CLI flags and conflict handling.
- `src/config.rs` for `[compression]` config parsing.
- `src/schema.rs` for the optional `compression` response object.
- `src/run.rs`, `src/start.rs`, `src/restart.rs`, and `src/tail.rs` for response wiring.
- `tests/integration.rs` for end-to-end contract coverage.

## Mode Model

Supported modes:

- `off`: no compression; omit `compression` field.
- `route`: choose a compressor from command argv and/or output shape.
- `errors`: extract error/failure/panic/traceback/assertion-oriented lines and nearby context.
- `tests`: summarize test output with failure focus.
- `logs`: deduplicate repeated lines and strip progress/noise where safe.
- `git`: summarize git status/diff/log-like output.
- `json`: summarize structure and types without preserving large values.
- `summary`: generic bounded heuristic summary.

`auto` is intentionally not supported because it is not an rtk public mode and would make the agent-exec surface ambiguous.

## Effective Mode Resolution

Resolution order:

1. CLI `--compress` / `--rtk`.
2. Config `[compression].default`.
3. Built-in `route`.

If both CLI flags are present with different modes, the command must fail with clap-style usage error exit code `2`. If both are present with the same mode, they may be accepted as equivalent aliases.

## Response Shape

Canonical raw fields remain unchanged:

- `stdout`
- `stderr`
- `stdout_range`
- `stderr_range`
- `stdout_total_bytes`
- `stderr_total_bytes`
- `encoding`
- log path fields where already present

When the resolved mode is not `off`, add:

```json
{
  "compression": {
    "mode": "route",
    "applied": true,
    "detected_kind": "cargo-test",
    "stdout": "FAILED: 2 tests\n...",
    "stderr": "",
    "stdout_original_bytes": 120000,
    "stderr_original_bytes": 0,
    "stdout_compressed_bytes": 512,
    "stderr_compressed_bytes": 0,
    "omitted": true,
    "strategy": ["failure-focus", "truncation"]
  }
}
```

`compression.stdout` and `compression.stderr` are not raw byte slices and must not reuse raw range metadata. The raw log paths remain the recovery path for full output.

## Compression Architecture

Add a local compression module, for example `src/compress.rs`, with pure functions over already-observed text:

- input: command argv, stdout excerpt, stderr excerpt, total byte counts, selected mode
- output: optional compression payload

The module should not spawn processes or read external tools. This keeps the feature deterministic, testable, and safe under the JSON-only stdout contract.

## Verification Strategy

Use integration tests as the primary verification because the feature is a CLI contract change.

Required behavior-bearing tests include:

- default no-config response includes `compression.mode = "route"`
- config default `off` omits `compression`
- CLI mode overrides config
- `--rtk` is equivalent to `--compress`
- conflicting flags produce usage exit code `2`
- `errors` extracts error-bearing lines
- `logs` deduplicates repeated lines
- raw fields remain present and raw when compressed output is added

Unit tests are useful for parser/config and compressor helpers, but do not replace integration coverage.

## Trade-offs

Keeping raw fields plus compressed fields increases JSON size slightly for small outputs. This is intentional because correctness and recoverability are more important than maximum compression. Users can still disable the field with `off`.

The first implementation should prefer conservative, deterministic heuristics over broad parsing complexity. More specialized compressors can be added later if tests demonstrate need.
