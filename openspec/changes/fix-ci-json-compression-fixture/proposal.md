---
change_type: implementation
priority: high
dependencies: []
references:
  - tests/integration.rs:7321
  - tests/integration.rs:7363
  - src/compress.rs:151
  - openspec/specs/agent-exec-run/spec.md:30
---

# Fix CI JSON Compression Fixture

**Change Type**: implementation

## Problem / Context

CI is failing in `compression_modes_have_behavior_for_errors_logs_and_json` because the JSON compression fixture can produce a compressed JSON shape string that is not smaller than the raw observed output. The current compression contract requires the expansion guard to suppress compressed text when it is greater than or equal to the raw observed output size.

The failing test currently expects `compression.stdout` to contain `object keys=2` unconditionally, but that expectation conflicts with the existing expansion guard requirement when the fixture is too small.

## Proposed Solution

Update the JSON compression regression coverage so it separately verifies:

- useful JSON compression applies when the JSON fixture is large enough for `json` mode to produce a smaller shape summary
- expansion guard still suppresses short JSON compressed views that would not be smaller than raw output

Do not weaken or bypass expansion guard behavior.

## Acceptance Criteria

- `cargo test --test integration compression_modes_have_behavior_for_errors_logs_and_json -- --nocapture` passes on CI-like Linux runners.
- JSON compression mode still demonstrates `compression.applied=true` and a `compression.stdout` shape summary containing `object keys=2` for a fixture whose raw JSON is larger than the summary.
- A short JSON fixture that would expand or equal raw output is covered by a regression test asserting `compression.applied=false`, `compression.strategy` includes `expansion-guard`, and `compression.stdout` is empty.
- Canonical raw `stdout` remains unchanged by compression in both applied and guarded cases.
- No production compression implementation is relaxed to satisfy the test.

## Explicit Completion Conditions

- `tests/integration.rs` contains deterministic regression coverage for both useful JSON compression and JSON expansion guard behavior.
- The JSON compression fixture in `compression_modes_have_behavior_for_errors_logs_and_json` is large enough that the compressed shape summary is smaller than the raw observed output on Linux CI.
- The focused integration test command passes locally.
- CI parity checks pass with `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` or `prek run -a`.

## Out of Scope

- Changing `src/compress.rs` expansion guard semantics.
- Changing the public compression JSON schema.
- Changing release version metadata.
