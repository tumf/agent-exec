## MODIFIED Requirements

### Requirement: head/tail 観測契約

`run` と `start` は head 観測（先頭 bytes）を返し、`tail` は tail 観測（末尾 bytes）を返さなければならない（MUST）。
`run`/`start`/`tail` は canonical field として `stdout` / `stderr` / `stdout_range` / `stderr_range` / `stdout_total_bytes` / `stderr_total_bytes` / `encoding` を返さなければならない（MUST）。

System, search, log, JSON, and env-like outputs routed through `route` compression must use structure-aware compact views when recognized (MUST). Compression must group large listings and search results, deduplicate repetitive logs, summarize JSON structure without large values, and mask secret-like values in env-like compressed views (MUST). Compression must not mutate or replace canonical raw observation fields (MUST NOT).

#### Scenario: search output is grouped by file

**Given**: observed `rg` or `grep` output contains many matching lines across files
**When**: search compression is applied
**Then**: matches are grouped by file
**And**: match counts are preserved
**And**: representative lines are bounded

#### Scenario: repeated logs are deduplicated

**Given**: observed log output contains repeated or timestamp-varied duplicate messages
**When**: log compression is applied
**Then**: duplicate messages are collapsed with counts when safe
**And**: error-bearing lines remain visible
**And**: progress noise is omitted or summarized

#### Scenario: JSON output is summarized by shape

**Given**: observed output contains a large JSON object, array, or NDJSON stream
**When**: JSON compression is applied
**Then**: object keys, value types, array lengths, or record counts are summarized
**And**: large scalar values are omitted from the compressed view
**And**: raw canonical stdout still contains the observed JSON text

#### Scenario: env-like compressed output masks secrets

**Given**: observed env-like output contains keys such as `TOKEN`, `PASSWORD`, or `SECRET`
**When**: env compression is applied
**Then**: `compression.stdout` masks secret-like values
**And**: non-secret keys may be summarized or grouped
**And**: raw canonical stdout remains unchanged
