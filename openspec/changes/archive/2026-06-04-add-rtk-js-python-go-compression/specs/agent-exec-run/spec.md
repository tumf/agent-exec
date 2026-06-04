## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

JS/TS, Python, and Go tool outputs routed through `route` compression must use language-family-specific compact views when the observed command and output shape are recognized (MUST). These compact views must preserve actionable diagnostics, failure identities, file/package/rule grouping, and final summaries while removing progress noise and redundant pass lists (MUST). Compression must not inject JSON flags or rewrite commands (MUST NOT).

#### Scenario: TypeScript and linter diagnostics are grouped

**Given**: observed output from `tsc`, `eslint`, or `biome` contains many diagnostics
**When**: language-family compression is applied
**Then**: diagnostics are grouped by file and rule or code when present
**And**: representative messages and locations are preserved
**And**: repeated or redundant diagnostic text is bounded

#### Scenario: Python tool output is compacted by structure

**Given**: observed output from `ruff`, `mypy`, `pytest`, or `pip` is large
**When**: Python compression is applied
**Then**: lint/type errors are grouped by rule or file when present
**And**: test failures are preserved while pass output is summarized
**And**: package lists are bounded and summarized

#### Scenario: Go output supports diagnostics and NDJSON events

**Given**: observed output from `go test`, `go build`, `go vet`, or `golangci-lint` contains text diagnostics or NDJSON events
**When**: Go compression is applied
**Then**: package-level summaries are preserved
**And**: failures or lint issues are grouped by package, file, rule, or test name
**And**: passing package/test noise is collapsed
