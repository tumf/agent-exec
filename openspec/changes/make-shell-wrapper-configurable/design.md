# Design: make-shell-wrapper-configurable

## Summary

This change introduces a small user config file and a shared shell-wrapper resolver so every command-string execution path uses the same launcher selection logic. The goal is to let users prefer wrappers such as `bash -lc` without making `run` and `--notify-command` diverge.

## Premises

- `--notify-command` already executes through a platform shell launcher.
- The requested direction is for the same wrapper setting to apply to both `run` command execution and `--notify-command`.
- CLI overrides should be ergonomic, so `--shell-wrapper` is a string.
- Config should be precise and machine-readable, so `config.toml` stores argv arrays.
- Docs must stay synchronized with the CLI contract, especially `README.md` and `skills/agent-exec/`.

## Config Contract

The config file lives at:

- `$XDG_CONFIG_HOME/agent-exec/config.toml`
- fallback: `~/.config/agent-exec/config.toml`

Example:

```toml
[shell]
unix = ["sh", "-lc"]
windows = ["cmd", "/C"]
```

Rules:

- keys are optional; absent values fall back to built-in platform defaults
- configured arrays must not be empty
- only the active platform's wrapper is used at runtime

## CLI Contract

New flags:

- `--config <PATH>`: load a specific config file
- `--shell-wrapper <PROGRAM AND FLAGS>`: override the shell wrapper for the current invocation

The CLI wrapper is parsed from a string into argv for the active platform. The parsed argv must not be empty.

## Resolution Order

The effective shell wrapper is resolved in this order:

1. CLI `--shell-wrapper`
2. file loaded via `--config`
3. default XDG config file
4. built-in platform default

This resolution path should be implemented once and reused by all command-string execution sites.

## Shared Execution Model

Both execution sites use the same launcher semantics:

- `run` command-string execution
- `--notify-command` completion delivery

In both cases, the implementation launches:

`<wrapper argv...> <command string>`

This keeps one mental model for users and one code path for maintainers.

## Failure Model

- missing config file is not an error; fall back to defaults
- invalid config syntax is a command failure with the normal JSON error contract
- empty wrapper configuration is a command failure with the normal JSON error contract
- notification delivery failure remains best effort and does not change the main job result

## Documentation Impact

The following docs should be updated in the same implementation:

- `README.md`
- `skills/agent-exec/SKILL.md`
- `skills/agent-exec/references/completion-events.md`
- any `run` usage examples that rely on hardcoded `sh -lc` or `cmd /C`

## Verification Notes

- integration tests should cover default wrapper behavior, XDG config loading, `--config` override, CLI override precedence, and invalid config handling
- at least one integration test should show that the same wrapper setting affects both `run` and `--notify-command`
- docs should be updated in the same change so the published contract matches the implemented behavior
