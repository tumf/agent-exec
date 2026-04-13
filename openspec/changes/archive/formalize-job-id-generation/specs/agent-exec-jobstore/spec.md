## ADDED Requirements

### Requirement: job_id の生成仕様

新規生成する `job_id` は 32 文字の小文字 16 進数文字列でなければならない（MUST）。エントロピー源は OS CSPRNG（128 bit 以上）でなければならない（MUST）。`short_job_id` はこの `job_id` の先頭 7 文字でなければならない（MUST）。

衝突検出（同名ディレクトリが既に存在する）時は最大 16 回まで再生成を試行しなければならない（MUST）。16 回連続で衝突した場合は `error.code="io_error"` の構造化エラーを返さなければならない（MUST）。無制限 loop をしてはならない（MUST NOT）。

#### Scenario: generated job_id is 32-char lowercase hex

**Given**: `agent-exec run -- echo hi` is executed
**When**: the JSON response is returned
**Then**: `job_id` matches `^[0-9a-f]{32}$`

#### Scenario: 16 consecutive collisions return io_error

**Given**: a fake RNG produces the same 16 bytes on 16 consecutive draws
**And**: a directory with that `job_id` already exists
**When**: `generate_job_id` is called
**Then**: the call returns an error mapped to `error.code="io_error"`
