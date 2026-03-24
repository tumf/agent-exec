# Change Proposal: add-shell-completions

## Problem / Context

`agent-exec` has 14+ subcommands, numerous flags, and constrained value sets
(e.g. `--state`, `--signal`, `--output-match-type`, `--output-stream`), yet
offers no shell completion support. Users must memorize or consult `--help`
for every interaction. Adding shell completions is a standard CLI quality-of-life
improvement and a prerequisite for dynamic job-ID completion.

## Proposed Solution

1. **Add `clap_complete` dependency** to `Cargo.toml`.
2. **Add a `completions <SHELL>` subcommand** that prints a static completion
   script to stdout for the requested shell (`bash`, `zsh`, `fish`,
   `powershell`). This follows the same JSON-only exemption pattern as the
   existing `schema` subcommand.
3. **Annotate constrained value arguments with `PossibleValues`** so
   `clap_complete` can enumerate them:
   - `--state`: `created`, `running`, `exited`, `killed`, `failed`, `unknown`
   - `--signal`: `TERM`, `INT`, `KILL`, `HUP`, `USR1`, `USR2`
   - `--output-match-type`: `contains`, `regex`
   - `--output-stream`: `stdout`, `stderr`, `either`
4. **Add `ValueHint` annotations** where appropriate:
   - `--cwd`, `--log`, `--env-file`, `--config`, `--notify-file`,
     `--output-file`: `ValueHint::FilePath` or `ValueHint::DirPath`
   - `<command...>` trailing args: `ValueHint::CommandWithArguments`

### Key design points

- The `completions` subcommand outputs plain text (not JSON), matching the
  `schema` subcommand precedent.
- The `<SHELL>` argument uses `ValueEnum` for compile-time validation.
- The hidden `_supervise` subcommand is excluded from completions by clap
  automatically (`hide = true`).
- This proposal covers **static** completions only. Dynamic job-ID completion
  is addressed in a separate proposal (`add-dynamic-job-completions`).

## Acceptance Criteria

- [ ] `agent-exec completions bash` outputs a non-empty Bash completion script
      (verified by integration test).
- [ ] `agent-exec completions zsh` outputs a non-empty Zsh completion script
      (verified by integration test).
- [ ] `agent-exec completions fish` outputs a non-empty Fish completion script
      (verified by integration test).
- [ ] `agent-exec completions powershell` outputs a non-empty PowerShell
      completion script (verified by integration test).
- [ ] `agent-exec completions invalid` exits with code 2 (usage error).
- [ ] `--state` tab-completes to valid state names in generated scripts.
- [ ] `--signal` tab-completes to common signal names in generated scripts.
- [ ] `--output-match-type` and `--output-stream` tab-complete to their
      respective valid values.
- [ ] `--cwd`, `--config`, `--env-file` hint at file/directory completion.
- [ ] `prek run -a` passes (fmt, clippy, tests).

## Out of Scope

- Dynamic job-ID completion (separate proposal: `add-dynamic-job-completions`).
- Nushell or Elvish completion support.
- Auto-installing completions (users run the command and redirect output).
- Changing the `schema` subcommand behavior.
