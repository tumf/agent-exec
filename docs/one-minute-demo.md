# One-minute demo

A normal synchronous subprocess keeps the caller waiting. `nohup` detaches the process, but leaves you to invent job IDs, status tracking, bounded logs, and cleanup.

`agent-exec` separates the observation deadline from the job lifetime. This demo observes for one second, returns a `job_id`, then reconnects to the same job after the original call has returned.

## Run it

Prerequisites: `agent-exec` and `jq` are on `PATH`.

```bash
START_FILE=$(mktemp)
agent-exec run --until 1 -- \
  sh -c 'echo started; sleep 3; echo finished' >"$START_FILE"

JOB_ID=$(jq -r .job_id "$START_FILE")
printf 'run returned: state=%s job_id=%s\n' \
  "$(jq -r .state "$START_FILE")" "$JOB_ID"
rm -f "$START_FILE"

agent-exec status "$JOB_ID" | jq '{job_id, state, exit_code}'
agent-exec tail "$JOB_ID" --tail-lines 10 | jq '{state, stdout}'
agent-exec wait "$JOB_ID" --until 5 | jq '{job_id, state, exit_code, stdout, stderr}'
agent-exec tail "$JOB_ID" --tail-lines 10 | jq '{state, stdout}'
```

Expected progression:

```text
run returned: state=running job_id=<persistent job id>
{"job_id":"<same id>","state":"running","exit_code":null}
{"state":"running","stdout":"started\n"}
{"job_id":"<same id>","state":"exited","exit_code":0,"stdout":"started\nfinished\n","stderr":""}
{"state":"exited","stdout":"started\nfinished\n"}
```

The exact timing can make `status` report `exited` on a busy machine, but the invariant is the same: every command uses the same persisted `job_id`, and the complete output remains available after the initial observation deadline. `wait` returns completion state, exit code, and bounded output; use `tail` later for repeated log retrieval.

## What this replaces

| Approach | Caller returns early | Stable job ID | Structured status | Bounded log retrieval | Reconnect later |
|---|---:|---:|---:|---:|---:|
| Synchronous subprocess | No | No | Exit code only | No | No |
| `nohup command &` | Yes | PID only | Manual | Manual | Manual |
| `agent-exec` | Yes | Yes | Yes | Yes | Yes |

An MCP client gets the same lifecycle through the `run`, `status`, `tail`, and `wait` tools. If the MCP request reaches its deadline, the managed job continues; retain the returned `job_id` and reconnect later.

## Verify success

The final `wait` object must contain:

- the same `job_id` returned by `run`
- `state: "exited"`
- `exit_code: 0`

The following `tail` object must contain both `started` and `finished` in `stdout`.

Use `agent-exec kill "$JOB_ID"` only when you explicitly want to cancel a running job.
