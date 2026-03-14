# Change Proposal: switch-notify-command-to-shell-string

## Problem/Context

`--notify-command` currently requires a JSON argv array and executes the sink without a shell. This is precise, but it is awkward for ad-hoc completion hooks because users often need shell features to transform the event JSON into CLI arguments.

The current docs already drift toward shell-oriented examples, especially for OpenClaw delivery. That mismatch makes the feature harder to explain and use.

The repository also supports Windows, so the change must preserve cross-platform behavior instead of assuming `/bin/sh` everywhere.

## Proposed Solution

Change `--notify-command` to accept a single command-line string instead of a JSON argv string.

At completion time, `agent-exec` will execute the command via the platform shell:

- Unix-like platforms: `sh -lc <command>`
- Windows: platform-default shell invocation (for example `cmd /C <command>`), with the exact launcher documented and tested

The completion event contract stays otherwise the same:

- event JSON is written to stdin
- `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and `AGENT_EXEC_EVENT_TYPE` are set
- notification failure remains best effort and does not change the main job state

Metadata and persisted completion delivery records will store the shell command string rather than an argv array.

## Acceptance Criteria

- `run` and the supervisor accept `--notify-command <string>` and no longer require JSON parsing
- the command sink executes through the platform shell, with coverage for Unix and Windows behavior
- `meta.json.notification.notify_command` persists the command as a string
- `completion_event.json.delivery_results` records the shell command target clearly
- `README.md` and files under `skills/agent-exec/` describe `--notify-command` as a shell command string and show readable ad-hoc examples
- integration tests verify event delivery via stdin, environment-variable availability, and non-destructive failure handling under the new contract

## Out of Scope

- adding new sink types beyond the existing command sink and file sink
- changing the completion event payload shape itself
- adding retry orchestration or durable delivery semantics to command sinks
