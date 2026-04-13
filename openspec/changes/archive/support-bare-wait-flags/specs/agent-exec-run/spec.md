## MODIFIED Requirements

### Requirement: run のジョブ生成と初回 inline output

`run` はジョブを起動し、既定では `--wait --until 10` 相当の待機予算内で観測できた stdout / stderr を初回レスポンスに含めなければならない（MUST）。`--wait` は人間向け CLI では裸指定だけで `true` として受理されなければならない（MUST）。後方互換のため `--wait true|false` も受け付けてよい（MAY）。`--no-wait` は `--wait false --until 0` のエイリアスであり、追加待機なしの launch-only 返却を明示的に選べなければならない（MUST）。

#### Scenario: run accepts bare wait flag

**Given**: a user executes `agent-exec run --wait -- echo hi`
**When**: CLI arguments are validated and the command runs
**Then**: the command succeeds instead of failing with a missing boolean value error
**And**: the effective wait behavior matches `agent-exec run --wait true -- echo hi`

#### Scenario: run preserves explicit boolean compatibility

**Given**: a user executes `agent-exec run --wait false -- echo hi`
**When**: CLI arguments are validated and the command runs
**Then**: the command succeeds
**And**: the effective wait behavior remains equivalent to `--no-wait`

### Requirement: run/start の観測責務

`run` と `start` は launch-only ではなく、既定では `--wait --until 10` 相当の待機予算内で初回レスポンスに inline output を含めなければならない（MUST）。`run` / `start` の人間向け CLI surface では `--wait` を裸指定だけで `true` として受理しなければならない（MUST）。`--wait true|false` は後方互換として受理してよい（MAY）。`--no-wait` は `--wait false --until 0` のエイリアスとして受け付けなければならない（MUST）。

#### Scenario: start accepts bare wait flag

**Given**: a job created by `agent-exec create -- sh -c "printf 'abc'"` exists
**When**: `agent-exec start --wait <job_id>` is executed
**Then**: the command succeeds instead of failing with a missing boolean value error
**And**: the effective wait behavior matches `agent-exec start --wait true <job_id>`

#### Scenario: start preserves explicit false compatibility

**Given**: a job created by `agent-exec create -- sh -c "sleep 60"` exists
**When**: `agent-exec start --wait false <job_id>` is executed
**Then**: the command succeeds
**And**: the effective wait behavior remains equivalent to `agent-exec start --no-wait <job_id>`
