## ADDED Requirements

### Requirement: completions subcommand

`agent-exec` MUST provide a `completions <SHELL>` subcommand that generates shell-specific completion scripts to stdout. `<SHELL>` MUST accept `bash`, `zsh`, `fish`, and `powershell` as valid values. Any other value MUST result in a usage error (exit code 2). The output MUST be a non-empty plain-text completion script (not JSON), following the same stdout exemption pattern as the `schema` subcommand.

#### Scenario: generate Bash completions

**Given**: `agent-exec completions bash` is executed
**When**: the command completes
**Then**: stdout contains a non-empty Bash completion script and exit code is 0

#### Scenario: generate Zsh completions

**Given**: `agent-exec completions zsh` is executed
**When**: the command completes
**Then**: stdout contains a non-empty Zsh completion script and exit code is 0

#### Scenario: invalid shell name is rejected

**Given**: `agent-exec completions invalid` is executed
**When**: the command completes
**Then**: exit code is 2 (usage error)

## MODIFIED Requirements

### Requirement: constrained option values

`--state` on the `list` subcommand MUST only accept `created`, `running`, `exited`, `killed`, `failed`, `unknown` as valid values. Invalid values MUST produce a usage error (exit code 2). `--signal` on the `kill` subcommand MUST enumerate common signal names (`TERM`, `INT`, `KILL`, `HUP`, `USR1`, `USR2`) as completion candidates while still accepting other values. `--output-match-type` MUST only accept `contains` or `regex`. `--output-stream` MUST only accept `stdout`, `stderr`, or `either`.

#### Scenario: invalid --state value rejected

**Given**: `agent-exec list --all --state bogus` is executed
**When**: the command completes
**Then**: exit code is 2 (usage error) and no JSON is written to stdout
