# Change Proposal: add-notify-set-command

## Problem/Context

`agent-exec run` can persist completion notification settings when a job is created, but there is no CLI to update those settings later.

- The current CLI only accepts `--notify-command` and `--notify-file` on `run`, even though notification configuration is already persisted in `meta.json`.
- Recent design discussion in this session established that `notify set` should stay focused on metadata updates, not immediate delivery side effects.
- The proposal must not absorb `--root` redesign work because `normalize-global-root-option` is already active and will define the global root syntax separately.
- Future job lifecycle expansion may add non-running pre-start states, so the proposal should avoid coupling notification updates to a specific job state.

## Proposed Solution

Add a `notify set` subcommand that updates the persisted command notification for any existing job and make completion delivery use the latest persisted notification configuration at dispatch time.

- Add `agent-exec notify set <JOB_ID> --command <COMMAND>` as a metadata update command.
- Allow the command for any existing job state; the subcommand updates metadata only and does not trigger immediate notification delivery.
- Preserve any existing `notify_file` configuration while replacing `notify_command` with the new shell command string.
- When a job later reaches a terminal state, completion delivery must read the current `meta.json.notification` instead of relying only on the values captured when `run` launched `_supervise`.
- Return a normal JSON success envelope that reflects the saved notification configuration.

## Acceptance Criteria

- `agent-exec notify set <job_id> --command 'cat >/tmp/event.json'` succeeds for any existing job and updates `meta.json.notification.notify_command`.
- `notify set` does not execute the configured shell command immediately, including when the target job is already terminal.
- Existing `notify_file` metadata is preserved when `notify_command` is updated.
- If `notify set` updates a job before it finishes, the later `job.finished` delivery uses the updated command string.
- A missing job returns the existing JSON error contract with `error.code = "job_not_found"`.

## Out of Scope

- Global `--root` syntax changes; those belong to `normalize-global-root-option`.
- Adding `notify clear`, `notify show`, `notify send`, or `notify replay`.
- Changing notification payload shape, delivery result schema, or shell wrapper semantics.
