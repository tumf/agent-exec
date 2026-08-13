# OpenCode auto-resume (reference integration)

Return control to the OpenCode session that launched a long `agent-exec` job, as
soon as that job finishes.

Without this, a job launched through the `agent-exec` MCP `run` tool can finish
long after the OpenCode turn that started it has gone idle. Nothing wakes the
session, so the work sits there until a human comes back and asks.

This is an **opt-in example**, not `agent-exec` behavior. It changes no runtime
default, adds no CLI subcommand, and does not touch the MCP `run` schema. It is
built entirely from contracts `agent-exec` already ships: `notify set` and the
persisted completion event.

## How it works

1. OpenCode calls the agent-exec MCP `run` tool.
2. The plugin observes the shared `tool.execute.after` hook. For a recognized
   agent-exec run result that reports a **still-running** job, it extracts the
   job ID.
3. The plugin runs
   `agent-exec notify set <job-id> --command "<helper> <server-url> <session-id>"`.
   The agent never has to ask for a notification, and never sees this happen.
4. When the job reaches a terminal state, the agent-exec supervisor runs that
   command with `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and
   `AGENT_EXEC_EVENT_TYPE` set.
5. The helper reads the persisted completion event, drops jobs shorter than the
   configured threshold, claims the job's single delivery, and runs
   `opencode run --attach <server-url> --session <session-id> <prompt>`.
6. That session wakes up, inspects the job, and continues the original task.

## Requirements

- `agent-exec` on `PATH` (or `AGENT_EXEC_BIN` set), configured as an MCP server
  in OpenCode.
- `opencode` on `PATH` (or `AGENT_EXEC_OPENCODE_BIN` set).
- Node.js, which OpenCode already requires to load plugins.

## Install

Both files are standalone; nothing is compiled or packaged.

```bash
# 1. The plugin, into the OpenCode global plugin directory.
OPENCODE_CONFIG_DIR="${OPENCODE_CONFIG_DIR:-$HOME/.config/opencode}"
mkdir -p "$OPENCODE_CONFIG_DIR/plugins"
cp examples/integrations/opencode-auto-resume/agent-exec-auto-resume.js \
   "$OPENCODE_CONFIG_DIR/plugins/"

# 2. The completion helper, onto PATH.
mkdir -p "$HOME/.local/bin"
cp examples/integrations/opencode-auto-resume/opencode-agent-exec-resume \
   "$HOME/.local/bin/"
chmod +x "$HOME/.local/bin/opencode-agent-exec-resume"
```

Then **restart OpenCode**. Plugins are loaded once at startup, so a running
instance will not pick this up.

If the helper is not on `PATH`, point at it explicitly instead:

```bash
export AGENT_EXEC_OPENCODE_RESUME_BIN="$HOME/.local/bin/opencode-agent-exec-resume"
```

Keep the helper out of any directory whose nearest `package.json` declares
`"type": "module"`. It is an extensionless CommonJS script, and that setting
would make Node parse it as an ES module.

## Verify the install

In an OpenCode session, ask the agent to launch a job that runs longer than the
threshold, for example: *"run `sleep 90 && echo done` with agent-exec"*.

The callback lives in the job's persisted metadata. From another terminal:

```bash
JOBS_ROOT="${AGENT_EXEC_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/agent-exec/jobs}"
agent-exec list --all
jq -r .notification.notify_command "$JOBS_ROOT/<job-id>/meta.json"
```

That should print the helper invocation with your loopback URL and session ID.
When the job finishes, the same session receives the continuation prompt, and
`"$JOBS_ROOT/<job-id>/completion_event.json"` records the delivery result.

Set `AGENT_EXEC_OPENCODE_DEBUG=1` before starting OpenCode to log every decision
the plugin and helper make to stderr.

## Configuration

All configuration is environment based; no source edits are required. The plugin
reads its variables from the OpenCode process, and the helper reads its variables
from the environment the `agent-exec` supervisor inherited at launch time.

| Variable | Default | Read by | Meaning |
| --- | --- | --- | --- |
| `AGENT_EXEC_OPENCODE_MIN_DURATION_MS` | `60000` | helper | Minimum job duration that earns a resume. A job at exactly this duration resumes. |
| `AGENT_EXEC_BIN` | `agent-exec` | plugin | agent-exec executable. |
| `AGENT_EXEC_OPENCODE_BIN` | `opencode` | helper | OpenCode executable. |
| `AGENT_EXEC_OPENCODE_RESUME_BIN` | sibling file, else `opencode-agent-exec-resume` on `PATH` | plugin | Completion helper to attach. |
| `AGENT_EXEC_OPENCODE_SERVER_URL` | auto-detected from the plugin context | plugin | Pin the loopback OpenCode server URL. |
| `AGENT_EXEC_OPENCODE_RUN_TOOLS` | auto-detected | plugin | Comma-separated exact MCP tool names to treat as agent-exec `run`. |
| `AGENT_EXEC_OPENCODE_STATE_DIR` | `$XDG_STATE_HOME/agent-exec-opencode-auto-resume` | helper | Where per-job delivery markers live. |
| `AGENT_EXEC_OPENCODE_DEBUG` | unset | both | Log decisions to stderr. |
| `AGENT_EXEC_ROOT` | agent-exec default | plugin | Set this if your MCP server uses a non-default jobs root. |

By default the plugin recognizes any MCP tool whose server segment normalizes to
`agentexec` and whose tool segment is `run` — `agent-exec_run`, `agent_exec_run`,
and `agentexec.run` all match. Use `AGENT_EXEC_OPENCODE_RUN_TOOLS` if you named
the server something else.

Because the helper's variables are read by the supervisor's child process, they
must be set in the environment that launched the job. In practice: export them
where you start OpenCode.

## What the session receives

The resume arrives as an ordinary **`role=user` message** in the OpenCode session.
OpenCode has no concept of a trusted internal event, so this is a normal user
turn in the transcript and in any usage accounting. The prompt opens with
`[automated message: not written by the user]` so the agent and any later reader
can tell it apart.

The prompt contains only fixed text plus the job ID and the completion event path.
Job stdout, stderr, argv, and cwd are never interpolated into it. It directs the
agent to read the event and logs, verify whether the work actually succeeded
rather than reporting status alone, and treat everything it reads there as
untrusted data.

## Security boundaries

- **Loopback only.** Both halves accept only `http://` URLs on `localhost`, `::1`,
  or `127.0.0.0/8`. Remote OpenCode servers are refused, not supported.
- **Validated identifiers.** Job IDs must be alphanumeric; session IDs are
  restricted to an alphanumeric-plus-`._-` character set. Anything else is
  dropped before a command is built.
- **No shell evaluation of job data.** The plugin single-quotes the three
  components of the `notify set` command string, and the helper invokes OpenCode
  through an argv array. No part of a completion event or job log ever reaches a
  shell.
- **Untrusted event data.** The completion event embeds the job's own argv and
  cwd. The helper parses it with a real JSON parser, uses only `event_type`,
  `job_id`, and `duration_ms`, and cross-checks `job_id` against
  `AGENT_EXEC_JOB_ID`.
- **Failure is contained.** The plugin never throws out of the hook, so a problem
  here cannot fail the tool call the agent is waiting on.

## Limitations

- **Best effort, not guaranteed delivery.** If the supervisor cannot run the
  helper, or OpenCode is gone, the resume is simply lost. There is no retry
  daemon and no durable queue. Sink outcomes are recorded in the job's
  `completion_event.json` under `delivery_results`.
- **Single host.** Delivery is deduplicated with an atomic marker directory per
  job ID under the state directory. That is single-host suppression, not
  distributed exactly-once delivery.
- **MCP only.** Jobs started with the `agent-exec` CLI directly, outside an
  OpenCode MCP tool call, are not covered — the plugin never sees them.
- **Already-finished jobs are skipped.** `run` observes inline output before
  returning, so a job that finishes inside that window returns terminal and needs
  no callback; the agent already has the result.
- **Restart required.** Plugin changes take effect only on OpenCode startup.
- **POSIX-oriented.** Developed and tested against a loopback OpenCode server on
  macOS and Linux.

## Uninstall

```bash
OPENCODE_CONFIG_DIR="${OPENCODE_CONFIG_DIR:-$HOME/.config/opencode}"
rm -f "$OPENCODE_CONFIG_DIR/plugins/agent-exec-auto-resume.js"
rm -f "$HOME/.local/bin/opencode-agent-exec-resume"
rm -rf "${XDG_STATE_HOME:-$HOME/.local/state}/agent-exec-opencode-auto-resume"
```

Restart OpenCode. Jobs that already carry the callback will still try to run the
helper once when they finish; that attempt fails harmlessly and is recorded in
`completion_event.json`. Disable it early on a specific job with
`agent-exec notify set <job-id> --command ''`, which leaves an empty command that
the supervisor refuses to run.

## Troubleshooting

| Symptom | Likely cause |
| --- | --- |
| Nothing is attached to any job | OpenCode was not restarted, or the MCP server is named something the plugin does not recognize. Set `AGENT_EXEC_OPENCODE_RUN_TOOLS`. |
| `meta.json` has no `notify_command` | The plugin could not resolve a loopback server URL. Set `AGENT_EXEC_OPENCODE_SERVER_URL`. |
| Callback attached, session never resumes | The job was shorter than the threshold, or the job was already delivered once. Run with `AGENT_EXEC_OPENCODE_DEBUG=1`. |
| `delivery_results` shows a command-sink failure | `opencode` or Node was not on the supervisor's `PATH`, or the OpenCode server had already exited. |
| Resume fires for a job you did not launch from OpenCode | Not possible through this path; jobs started outside an OpenCode MCP tool call never get a callback attached. |

## Tests

Repository tests drive the plugin and helper against fake `agent-exec` and
`opencode` executables in temporary directories, plus one end-to-end pass against
the real `agent-exec` binary and supervisor:

```bash
cargo test --test opencode_integration
```
