## MODIFIED Requirements

### Requirement: run のジョブ生成と初回 inline output

`run` はジョブを起動し、既定では `--wait --until 10` 相当の待機予算内で観測できた stdout / stderr を初回レスポンスに含めなければならない（MUST）。`--wait` は人間向け CLI では裸指定だけで `true` として受理されなければならない（MUST）。後方互換のため `--wait true|false` も受け付けてよい（MAY）。`--no-wait` は `--wait false --until 0` のエイリアスであり、追加待機なしの launch-only 返却を明示的に選べなければならない（MUST）。

10 秒以内にジョブが終端状態に到達した場合、初回レスポンスに `exit_code`・`finished_at`・`duration_ms` を必ず含めなければならない（MUST）。signal によって終了した場合は `signal`（POSIX signal 名、例 `SIGTERM`）も必ず含めなければならない（MUST）。終端状態に到達しなかった場合は `exit_code` / `finished_at` / `signal` / `duration_ms` のいずれも含めてはならない（MUST NOT）。`duration_ms` は `finished_at - started_at` をミリ秒で表した非負整数でなければならない（MUST）。

#### Scenario: run inline returns exit_code and duration_ms on short exit

**Given**: a user executes `agent-exec run -- sh -c "exit 7"`
**When**: the inline observation completes within the default budget
**Then**: the JSON includes `exit_code=7`, `finished_at`, and `duration_ms`
**And**: `signal` is absent

#### Scenario: run inline includes signal on signal-terminated exit

**Given**: a user executes `agent-exec run -- sh -c "kill -TERM $$"` on a Unix-like platform
**When**: the inline observation completes
**Then**: the JSON includes `signal`

#### Scenario: run inline omits completion fields for long jobs

**Given**: a user executes `agent-exec run -- sh -c "sleep 30"` with the default 10-second budget
**When**: the inline observation returns before the job exits
**Then**: the JSON omits `exit_code`, `finished_at`, `signal`, and `duration_ms`
