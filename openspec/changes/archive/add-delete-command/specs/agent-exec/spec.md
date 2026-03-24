## ADDED Requirements

### Requirement: delete subcommand

`agent-exec` MUST provide a `delete` subcommand for explicit job-directory removal. `delete` MUST support exactly one of `delete <job_id>` or `delete --all` (MUST). `delete --dry-run` MAY be combined with either mode to report actions without removing directories (MAY).

#### Scenario: delete removes one explicit job

Given an existing job `<job_id>` is in state `created`, `exited`, `killed`, or `failed`
When `agent-exec delete <job_id>` is executed
Then the job directory is removed
And later `agent-exec status <job_id>` returns `ok=false` with `error.code="job_not_found"`

#### Scenario: delete rejects a running job

Given an existing job `<job_id>` is in state `running`
When `agent-exec delete <job_id>` is executed
Then the command returns `ok=false`
And the job directory remains present

#### Scenario: delete all removes finished jobs only in current cwd

Given terminal jobs exist for the caller's current working directory and for a different working directory
When `agent-exec delete --all` is executed from the first working directory
Then only jobs whose persisted `meta.json.cwd` matches that current working directory are eligible for deletion
And only terminal jobs in that scoped set are deleted

#### Scenario: delete dry-run preserves directories

Given at least one job matches the chosen delete mode
When `agent-exec delete --dry-run --all` is executed
Then no job directories are removed
And the response reports which jobs would be deleted

### Requirement: delete response payload

`delete` MUST return a machine-readable response containing at least `root`, `dry_run`, `deleted`, `skipped`, and `jobs` (MUST). Each `jobs` entry MUST include at least `job_id`, `state`, `action`, and `reason` (MUST).

#### Scenario: delete returns per-job actions

Given `agent-exec delete --all` is executed
When the command completes
Then the response includes aggregate deleted/skipped counts
And each matched or inspected job reported in `jobs` includes its action and reason

## MODIFIED Requirements

### Requirement: README の利用導線

README は `run/status/tail/wait/kill/list/delete` を対象にしたコピペ可能な使用例を含めなければならない（MUST）。README は stdout が JSON-only であり、stderr が診断ログであることを明記しなければならない（MUST）。README は `delete` が cwd-scoped finished-job cleanup を提供し、`gc` は age-based root-wide cleanup のままであることを明記しなければならない（MUST）。

#### Scenario: README の delete コマンド例

Given リポジトリの `README.md` を読む
When 利用例セクションを確認する
Then `delete <job_id>` と `delete --all` の例が含まれる
And `delete` と `gc` の役割の違いが説明されている
