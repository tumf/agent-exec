## MODIFIED Requirements

### Requirement: run command-string execution wrapper configuration

When `run` executes commands through a shell wrapper, the effective wrapper must still be resolved from CLI overrides, config files, or built-in defaults (MUST). On Unix-like platforms, single-string command mode may continue to run as a shell command string, but argv-style invocations with more than one argument must use the resolved shell wrapper only as a launch handoff and must replace the wrapper process with the target argv workload via `exec` semantics (MUST).

#### Scenario: argv-style run uses shell-wrapper exec handoff on Unix

Given a Unix-like platform with the default shell wrapper
When `agent-exec run -- cflx run` is executed
Then the job still launches through the resolved shell wrapper
And the wrapper replaces itself with the target argv workload for completion tracking

#### Scenario: single-string run preserves shell-string semantics

Given a Unix-like platform with the default shell wrapper
When `agent-exec run -- 'echo hello && echo world'` is executed
Then the job runs as a shell command string through the resolved wrapper
And shell syntax remains available to that command string
