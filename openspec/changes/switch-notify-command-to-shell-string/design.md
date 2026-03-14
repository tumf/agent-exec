# Design: switch-notify-command-to-shell-string

## Summary

This change redefines the command notification sink from a shell-free argv contract to a shell-command-string contract. The main goal is to make ad-hoc completion hooks much easier to write while preserving the existing completion event payload, stdin delivery, and best-effort semantics.

## Current State

- CLI accepts `--notify-command <JSON_ARGV>`
- `src/main.rs` parses the string into `Vec<String>` before execution
- `src/schema.rs` persists the command sink as `Option<Vec<String>>`
- docs must explain JSON-in-a-string examples, which is hard to read and easy to misuse

## Proposed Contract

### CLI and persistence

- `--notify-command` accepts a single shell command string
- runtime options carry `Option<String>` instead of `Option<Vec<String>>`
- persisted notification metadata stores the command string exactly as configured

### Execution model

- Unix-like platforms launch `sh -lc <command>`
- Windows launches the configured command through the platform-default shell
- event JSON is still written to stdin
- `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and `AGENT_EXEC_EVENT_TYPE` remain available to the shell command

### Failure model

- shell startup failure or non-zero shell exit counts as notification delivery failure
- command sink failure remains observational only and must not alter the job's terminal state
- `completion_event.json.delivery_results` records the attempted command string and error text

## Trade-offs

### Benefits

- much better ad-hoc usability
- README and skill examples become shorter and more realistic
- users can directly rely on shell expansions, pipes, redirection, and command substitution

### Costs

- the command sink is no longer shell-free
- quoting rules become shell-dependent
- cross-platform behavior must be explicit because Unix and Windows shells differ

## Platform Strategy

To preserve existing Windows support, the implementation should use a platform shell rather than hardcoding `sh -lc` everywhere.

- Unix-like: `sh -lc`
- Windows: default supported shell launcher documented in code and docs

This keeps the user-facing contract simple (`--notify-command` is always a command string) while allowing platform-appropriate execution internally.

## Verification Notes

- Unix integration tests should verify a simple shell command captures stdin-delivered event JSON
- shell failure tests should verify `completion_event.json.delivery_results` records the failure while `status` still reports the original terminal state
- docs and skill references should be updated in the same change so the published contract matches the implementation
