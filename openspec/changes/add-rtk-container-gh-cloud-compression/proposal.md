---
change_type: implementation
priority: medium
dependencies:
  - add-rtk-compression-routing
  - add-rtk-system-log-json-compression
references:
  - src/compress.rs
  - tests/integration.rs
  - rtk-ai/rtk/src/cmds/cloud
  - rtk-ai/rtk/src/cmds/git/gh_cmd.rs
  - rtk-ai/rtk/src/cmds/git/glab_cmd.rs
---

# Add RTK-style Container GitHub Cloud Compression

**Change Type**: implementation

## Problem/Context

Containers, Kubernetes, GitHub/GitLab CLI, and AWS outputs commonly include large tables, JSON documents, logs, markdown bodies, progress output, and policy documents. RTK compresses these by table parsing, status prioritization, JSON field pruning, and failure-first summaries.

## Proposed Solution

Add specialized compressors for observed output from Docker, Docker Compose, kubectl, gh, glab, curl/wget, AWS CLI, and psql-like table output where safe.

## Acceptance Criteria

- Container and Kubernetes table outputs preserve names, status, readiness, age, image, and error states while pruning less useful columns.
- Container/Kubernetes logs reuse log compression and preserve failures.
- `gh`/`glab` PR/issue/run outputs summarize identity, state, checks, labels, and relevant body sections.
- AWS JSON/table outputs summarize high-value fields and omit policy documents/secrets/large nested values.
- curl/wget progress is stripped while preserving HTTP/result/error context.

## Explicit Completion Conditions

Fixture-backed tests prove compact summaries for representative Docker, kubectl, gh, glab, AWS, curl/wget, and table outputs without raw-field mutation.

## Dependencies

Requires `add-rtk-compression-routing` and benefits from generic table/log/JSON helpers in `add-rtk-system-log-json-compression`.

## Out of Scope

- Network calls or external service credentials in tests.
- Rewriting commands to request JSON output.
