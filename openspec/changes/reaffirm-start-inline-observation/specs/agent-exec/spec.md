## ADDED Requirements

### Requirement: start は run と同じ inline 観測契約を共有する

`agent-exec start <job_id>` の既定は `run` と同じ inline 観測契約でなければならない（MUST）:
- `--wait` 既定 `true`
- `--until` 既定 `10` 秒
- `--max-bytes` 既定 `65536` バイト
- 返却 field は `stdout`・`stderr`・`stdout_range`・`stderr_range`・`stdout_total_bytes`・`stderr_total_bytes`・`encoding`・`stdout_log_path`・`stderr_log_path`、および短命終了時は `exit_code`・`finished_at`・`signal`（signal 終了時）・`duration_ms`

`create` → `start` の呼び出しフローは、`run` 単独呼び出しと同じ往復回数で起動＋観測を完結できなければならない（MUST）。`start` を launch-only にしてはならない（MUST NOT）。

#### Scenario: start default matches run default

**Given**: `agent-exec create -- sh -c "printf 'abc'"` で作成した job
**When**: `agent-exec start <job_id>` を実行する
**Then**: 返却 JSON の shape は `agent-exec run -- sh -c "printf 'abc'"` と同じ field 集合を持つ
