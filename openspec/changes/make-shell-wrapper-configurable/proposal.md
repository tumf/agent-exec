# Change Proposal: make-shell-wrapper-configurable

## Problem/Context

`agent-exec` currently hardcodes the shell launcher used for command-string execution.

- `run` command execution is being discussed to use a shell wrapper like `sh -lc` on Unix.
- `--notify-command` already uses a hardcoded platform shell launcher.
- Users need a way to override that launcher, for example to prefer `bash -lc` instead of `sh -lc`.
- The repository does not yet define a user config file contract, so the override behavior, config path, and CLI precedence all need to be specified.
- Repository-facing docs must stay aligned with the CLI contract, including `README.md` and the built-in `skills/agent-exec/` documentation.

This proposal keeps the launcher setting shared across all command-string execution paths so `run` and `--notify-command` do not drift.

## Proposed Solution

Add a small `agent-exec` configuration file and matching CLI overrides for the shell wrapper used to execute command strings.

- Read config from XDG config locations using `config.toml`.
- Add `--config <PATH>` to load a specific config file.
- Add `--shell-wrapper <PROGRAM AND FLAGS>` as a per-invocation override.
- Store shell wrapper values in config as argv arrays for precision.
- Parse the CLI override as a string for usability, then normalize it to argv before execution.
- Apply the resolved wrapper uniformly to both `run` command-string execution and `--notify-command` delivery.
- Document precedence, defaults, validation, and examples in `README.md` and `skills/agent-exec/`.

The default wrapper remains platform-specific when no override is configured:

- Unix-like platforms: `sh -lc`
- Windows: `cmd /C`

The precedence order is:

1. `--shell-wrapper`
2. `--config <PATH>` file
3. default XDG config file
4. built-in platform default

## Acceptance Criteria

- `agent-exec` supports config loading from `$XDG_CONFIG_HOME/agent-exec/config.toml` and falls back to `~/.config/agent-exec/config.toml` when `XDG_CONFIG_HOME` is unset.
- `run` accepts `--config <PATH>` and `--shell-wrapper <PROGRAM AND FLAGS>`.
- The configured shell wrapper applies to both `run` command-string execution and `--notify-command` execution.
- Config stores shell wrappers as arrays, with platform-specific keys for Unix and Windows.
- CLI string overrides are parsed and validated before execution; empty wrappers are rejected.
- `README.md` and `skills/agent-exec/` describe the shared shell wrapper setting, config path, precedence, and examples.
- Integration coverage verifies default behavior, config-file override behavior, CLI override precedence, and stable failure handling for invalid config or invalid wrapper settings.

## Out of Scope

- Supporting multiple config formats.
- Adding separate wrapper settings for `run` and `--notify-command`.
- Changing completion event payload contents beyond any wrapper metadata explicitly needed for reproducibility.
