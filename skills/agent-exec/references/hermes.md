# Hermes Agent Integration

Use this reference when Hermes launches a detached `agent-exec` job and must receive one completion event without repeated bounded waits.

## Current contract

Hermes does not automatically subscribe to the inner detached job created by `agent-exec run`.

`terminal(background=true, notify_on_complete=true)` watches the exact process started by the terminal tool. If that process is `agent-exec run`, it ends after returning the managed `job_id`; the workload continues under the detached supervisor. The terminal completion event therefore means only that the launcher returned, not that the managed workload finished.

The current Hermes CLI has no `hermes notify` command. Do not use historical examples that invoke it. Verify available commands with `hermes --help` before documenting a Hermes callback.

## One-watcher pattern

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

1. Start `agent-exec run` using the Hermes terminal tool as documented for this environment.
2. Read the returned JSON and retain the inner `job_id`.
3. If `state` is already terminal, verify the result directly; do not create a watcher.
4. If `state` is non-terminal and no real completion sink is armed, start one background `wait --forever` watcher with `notify_on_complete=true`.
5. Record the Hermes background `session_id` so the watcher is not duplicated.
6. When notified, verify the job output and requested artifact. Do not equate watcher exit with successful workload completion; check `state` and `exit_code`.

## Avoid double-background confusion

Do not assume this launch is sufficient:

```text
terminal(
  command="agent-exec run -- <command>",
  background=true,
  notify_on_complete=true,
)
```

Its completion notification normally reports only that `agent-exec run` returned its JSON envelope. Parse the inner `job_id`, then attach the single watcher above.

## Existing notification sinks

If the `agent-exec run` response truthfully reports that a completion sink is armed, use that sink and do not add a duplicate watcher. A configured sink must be evidenced by the response or persisted job metadata; client type or skill text is not evidence.

The watcher is the fallback for current Hermes operation when no real sink is armed. It is tied to the Hermes runtime managing the background process; it is not a durable cross-restart delivery service.

## Verification checklist

- `hermes --help` confirms the commands used by the procedure.
- `agent-exec wait --help` confirms `--forever` semantics.
- The launcher's JSON provides the authoritative inner `job_id`.
- Exactly one watcher exists for that job.
- Completion handling checks terminal `state`, `exit_code`, and persisted logs/artifacts.
