## ADDED Requirements

### Requirement: inline output の既定 max-bytes

`run` および `start` の `--max-bytes` 既定値は `65536` バイト（64 KiB）でなければならない（MUST）。`POST /exec` の `max_bytes` も同じ既定値を用いなければならない（MUST）。この既定値を変更する場合は `schema_version` の minor または major を bump しなければならない（MUST）。

#### Scenario: run uses default 64 KiB max-bytes

**Given**: a command whose stdout exceeds 128 KiB
**When**: `agent-exec run -- <cmd>` is executed without `--max-bytes`
**Then**: `stdout_range[1] - stdout_range[0]` is at most `65536`
**And**: `stdout_total_bytes` reflects the full output size
