---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/compress.rs
  - tests/integration.rs
  - rtk-ai/rtk/src/cmds/system
  - rtk-ai/rtk/src/core/filter.rs
---

# Add RTK-style System Log JSON Compression

**Change Type**: implementation

## Problem/Context

System commands, file/search outputs, logs, and JSON payloads can be extremely large. RTK handles these through tree compression, grouping, code filtering, log deduplication, and JSON shape extraction.

## Proposed Solution

Add specialized compressors for `ls`, `tree`, `find`, `grep`, `rg`, `cat`/file reads, `head`, `tail`, `wc`, logs, JSON/JQ-like output, and env-like output.

## Acceptance Criteria

- Directory and file listings are grouped by directory and capped.
- Search results are grouped by file with match counts and representative lines.
- Repeated logs are deduplicated, including timestamp-normalized duplicates where safe.
- JSON outputs summarize structure, keys, array length, and types without large values.
- Env-like outputs mask secret-like values in compression views.

## Explicit Completion Conditions

Representative fixture tests demonstrate smaller compressed outputs for large lists, search results, logs, JSON arrays/objects, and env-like output while preserving enough context for an agent to act.

## Dependencies

Previously required `add-rtk-compression-routing`, which has already been archived.

## Out of Scope

- Actual file reading beyond the already observed command output.
- Changing raw log persistence.
