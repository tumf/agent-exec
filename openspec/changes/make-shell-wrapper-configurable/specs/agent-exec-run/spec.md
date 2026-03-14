## MODIFIED Requirements

### Requirement: run completion notification configuration

`run` must support completion notification sinks (MUST). `--notify-command <command>` and `--notify-file <path>` must be accepted (MUST). `--notify-command` must be interpreted as a single shell command string rather than a JSON argv array (MUST). Notification configuration must be persisted in job metadata (MUST). The shell wrapper used for command-string execution must be resolved from CLI overrides, config files, or built-in defaults and shared with `run` command execution (MUST).

#### Scenario: shell wrapper setting applies to notify-command and run command execution

Given a shell wrapper is configured via CLI or config
When `agent-exec run` executes a command string and later delivers a `--notify-command` completion hook
Then both execution paths use the same resolved shell wrapper

### Requirement: command sink and file sink delivery contract

The `--notify-command` sink must execute through the resolved shell wrapper and receive the completion event JSON on stdin (MUST). `AGENT_EXEC_EVENT_PATH`, `AGENT_EXEC_JOB_ID`, and `AGENT_EXEC_EVENT_TYPE` must be added to the sink environment (MUST). The `--notify-file` sink must append one completion event JSON line as NDJSON and create parent directories when needed (MUST).

#### Scenario: notify-command uses CLI shell-wrapper override

Given `agent-exec run --shell-wrapper "bash -lc" --notify-command 'cat > /tmp/event.json' -- '<command-string>'` is executed on a Unix-like platform
When the job finishes
Then the notify-command sink is launched through the CLI-provided wrapper
And completion event delivery still uses stdin and the documented environment variables

### Requirement: run command-string execution wrapper configuration

When `run` executes command strings through a shell wrapper, the effective wrapper must be configurable (MUST). `run` must support a default XDG config file, `--config <PATH>`, and `--shell-wrapper <PROGRAM AND FLAGS>` (MUST). The effective wrapper must be selected in precedence order of CLI override, explicit config path, default XDG config, then built-in platform default (MUST).

#### Scenario: config file overrides built-in wrapper

Given `$XDG_CONFIG_HOME/agent-exec/config.toml` contains a wrapper for the active platform
When `agent-exec run -- '<command-string>'` is executed
Then the job command is launched through the configured wrapper instead of the built-in default

#### Scenario: CLI shell-wrapper overrides config file

Given the config file defines one shell wrapper
When `agent-exec run --config /tmp/agent-exec.toml --shell-wrapper "bash -lc" -- '<command-string>'` is executed on a Unix-like platform
Then the job command is launched through the CLI-provided wrapper
And the config-defined wrapper is not used for that invocation

#### Scenario: invalid shell-wrapper configuration fails before execution

Given the selected config file contains an empty wrapper array for the active platform
When `agent-exec run -- '<command-string>'` is executed
Then `run` fails with the standard JSON error contract
And no job process is started
