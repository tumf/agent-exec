# Hermes Agent Integration

Use this reference when Hermes launches a detached `agent-exec` job and must receive one completion event without repeated bounded waits.

## Current contract

Hermes does not automatically subscribe to the inner detached job created by `agent-exec run`.

`terminal(background=true, notify_on_complete=true)` watches the exact process started by the terminal tool. If that process is `agent-exec run`, it ends after returning the managed `job_id`; the workload continues under the detached supervisor. The terminal completion event therefore means only that the launcher returned, not that the managed workload finished.

The current Hermes CLI has no `hermes notify` command. Do not use historical examples that invoke it. Verify available commands with `hermes --help` before documenting a Hermes callback.

## Preferred: arm a completion sink at launch

Prefer a launch-time completion sink over any watcher. `run` persists the sink before the workload starts, and the terminal dispatcher runs it once when the job finishes.

The destination is **request-scoped**. Completion sinks do not inherit the managed child's environment, so `--env` / `env` routing never reaches the hook. Embed the target assignment in the sink command itself; the sink runs under the configured shell wrapper, so a leading assignment is valid:

```text
run(
  command=["./long-task.sh", "--verbose"],
  notify_command="HERMES_HOME=/absolute/profile/home HERMES_BIN=/absolute/path/hermes HERMES_NOTIFY_TARGET=slack:C0123:171234.0001 /absolute/path/skills/agent-exec/scripts/hermes-notify-hook",
)
```

For managed Hermes profiles, set `HERMES_HOME` explicitly to that agent's runtime home. Completion sinks do not reliably inherit the launching Hermes process environment. Set `HERMES_BIN` explicitly when `hermes` may be absent from the sink's `PATH`. Keep the assignments and paths in the persisted `notify_command`; do not pass them through the managed child's `env`.

CLI equivalent:

```bash
agent-exec run \
  --notify-command "HERMES_NOTIFY_TARGET='telegram:12345:678' /absolute/path/skills/agent-exec/scripts/hermes-notify-hook" \
  -- ./long-task.sh --verbose
```

`HERMES_NOTIFY_TARGET` is `platform:chat_id[:thread_id]` and must be the destination of the request being served. The hook never discovers, caches, or reuses a route: an absent or malformed target makes it exit non-zero instead of delivering somewhere else.

On completion the hook sends exactly one short notification through the current Hermes CLI:

```text
[AUTO: agent-exec completion event]
execution: <job_id>
event: completed

完了イベント `<completion_event.json path>` を確認し、元の作業を再開してください。
```

The `[AUTO: ...]` prefix and explicit follow-up instruction let a Hermes continuation classifier recognize this assistant-history message as resumable work. This is a prompt-level convention, not a fixed parser contract.

No LLM turn is started, and no command output or credential is included. Read `completion_event.json` at `AGENT_EXEC_EVENT_PATH`, or call `status`/`tail`, for the terminal `state`, `exit_code`, and logs.

### `armed` means persisted, not delivered

When `run` returns while the job is still running, the response may include `notification.state="armed"` with `polling_required=false`. That proves only that the sink is persisted in job metadata and will be dispatched at terminal time. It is not evidence that Slack, Telegram, or any downstream destination received the message; downstream delivery results are recorded separately in the completion event. Do not add a duplicate watcher for an armed sink, and do not treat `armed` as delivery confirmation.

If the hook exits non-zero, the delivery failure is recorded in the completion event and the workload's terminal state is unchanged.

## Fallback: one-watcher pattern

Use this only when no completion sink is persisted for the job.

After recovering the inner `job_id`, start exactly one Hermes-managed watcher:

```text
terminal(
  command="agent-exec wait --forever <job_id>",
  background=true,
  notify_on_complete=true,
)
```

This gives Hermes one process whose lifetime matches the managed job:

1. `agent-exec wait --forever` stays alive while the job is non-terminal.
2. It does not stop or own the workload.
3. It exits with the canonical terminal response when the job finishes.
4. Hermes emits one background-process completion event for that watcher.
5. On that event, inspect the returned terminal state and logs, then continue the original task.

Do not repeat `wait --until`, `status`, or `tail` merely to detect completion after this watcher is armed. Use `status` or `tail` only for an explicit progress request or abnormal-job diagnosis.

## Correct launch sequence

When shell work may outlive the terminal call:

1. Start `agent-exec run` using the Hermes terminal tool as documented for this environment, arming the completion sink above whenever a delivery destination is known.
2. Read the returned JSON and retain the inner `job_id`.
3. If `state` is already terminal, verify the result directly; do not create a watcher.
4. If `state` is non-terminal and no real completion sink is armed, start one background `wait --forever` watcher with `notify_on_complete=true`.
5. Record the Hermes background `session_id` so the watcher is not duplicated.
6. When notified, verify the job output and requested artifact. Do not equate watcher exit or sink dispatch with successful workload completion; check `state` and `exit_code`.

## Avoid double-background confusion

Do not assume this launch is sufficient:

```text
terminal(
  command="agent-exec run -- <command>",
  background=true,
  notify_on_complete=true,
)
```

Its completion notification normally reports only that `agent-exec run` returned its JSON envelope. Parse the inner `job_id`, then arm a sink or attach the single watcher above.

## Existing notification sinks

If the `agent-exec run` response truthfully reports that a completion sink is armed, use that sink and do not add a duplicate watcher. A configured sink must be evidenced by the response or persisted job metadata; client type or skill text is not evidence.

The watcher is the fallback for current Hermes operation when no real sink is armed. It is tied to the Hermes runtime managing the background process; it is not a durable cross-restart delivery service.

## Verification checklist

- `hermes --help` confirms the commands used by the procedure, including `hermes send`.
- `agent-exec wait --help` confirms `--forever` semantics.
- The launcher's JSON provides the authoritative inner `job_id`.
- The persisted `notify_command` carries this request's `HERMES_NOTIFY_TARGET`.
- Exactly one completion mechanism exists for that job: an armed sink or one watcher, never both.
- Completion handling checks terminal `state`, `exit_code`, and persisted logs/artifacts.
