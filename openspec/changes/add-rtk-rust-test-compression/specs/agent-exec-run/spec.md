## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

Rust build/test outputs and common test-runner outputs routed through `route` or explicit `tests`/`errors` compression must focus on failures, diagnostics, and summaries rather than passing-test or progress noise (MUST). Compression must preserve enough failure and diagnostic context to identify the failing test, assertion or panic message, diagnostic code, file location, and primary error text (MUST). Compression must not replace canonical raw observation fields (MUST NOT).

#### Scenario: cargo diagnostics preserve actionable error context

**Given**: a `cargo build`, `cargo check`, or `cargo clippy` output contains compiler diagnostics
**When**: route compression classifies the command as a Rust diagnostic command
**Then**: `compression.stdout` or `compression.stderr` preserves diagnostic code or severity
**And**: file and line information is preserved when present
**And**: compile progress noise is omitted or aggregated

#### Scenario: cargo test focuses on failures

**Given**: a `cargo test` output contains many passing tests and one or more failures
**When**: test compression is applied
**Then**: failing test names and failure details are preserved
**And**: passing tests are summarized by count rather than listed individually
**And**: bounded panic/backtrace context is preserved when present

#### Scenario: generic test runners summarize pass output

**Given**: a common test runner output contains pass/fail/skip lines
**When**: route compression classifies it as test output
**Then**: final counts are preserved
**And**: failure sections are preserved
**And**: passing test lists are collapsed into a compact summary
