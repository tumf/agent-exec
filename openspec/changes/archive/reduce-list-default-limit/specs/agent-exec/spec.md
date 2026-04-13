## MODIFIED Requirements

### Requirement: list の件数制限と truncated フラグ

`list` の `--limit <N>` は返却する件数の上限を指定し、既定値は `50` でなければならない（MUST）。`--limit 0` は「明示的無制限」を意味し受理しなければならない（MUST）。

レスポンスには `truncated: bool` を必ず含めなければならない（MUST）。制限に達し未返却のジョブが残っている場合 `truncated=true`、それ以外は `false` でなければならない（MUST）。

#### Scenario: list default returns up to 50 jobs with truncated=true

**Given**: 60 jobs exist under the caller's cwd
**When**: `agent-exec list` is executed
**Then**: `jobs` has length `50`
**And**: `truncated` is `true`

#### Scenario: list --limit 0 returns all jobs

**Given**: 60 jobs exist under the caller's cwd
**When**: `agent-exec list --limit 0` is executed
**Then**: `jobs` has length `60`
**And**: `truncated` is `false`
