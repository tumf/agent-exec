## Implementation Tasks

- [ ] Task 1: Add `clap_complete` dependency to `Cargo.toml` (verification: `cargo build` succeeds; `Cargo.toml` lists `clap_complete = "4"`)
- [ ] Task 2: Define `Shell` enum with `ValueEnum` derive for `bash`/`zsh`/`fish`/`powershell` in `src/main.rs` (verification: `cargo check` passes)
- [ ] Task 3: Add `Completions { shell: Shell }` variant to the `Commands` enum in `src/main.rs` with help text (verification: `agent-exec completions --help` shows usage)
- [ ] Task 4: Implement `completions` handler that calls `clap_complete::generate()` with the appropriate shell generator and writes to stdout (verification: `agent-exec completions bash` outputs non-empty script)
- [ ] Task 5: Convert `--state` in `list` to use `ValueEnum` enum (`created`, `running`, `exited`, `killed`, `failed`, `unknown`) instead of `Option<String>` (verification: `agent-exec list --state invalid` exits with code 2; `cargo test --all` passes)
- [ ] Task 6: Convert `--signal` in `kill` to use `PossibleValues` or constrained parser listing `TERM`, `INT`, `KILL`, `HUP`, `USR1`, `USR2` (verification: completion script contains signal names; existing tests pass)
- [ ] Task 7: Convert `--output-match-type` to use `PossibleValues` (`contains`, `regex`) across `run`, `create`, `notify set` (verification: completion script contains these values)
- [ ] Task 8: Convert `--output-stream` to use `PossibleValues` (`stdout`, `stderr`, `either`) across `run`, `create`, `notify set` (verification: completion script contains these values)
- [ ] Task 9: Add `ValueHint::DirPath` to `--cwd` arguments, `ValueHint::FilePath` to `--config`, `--env-file`, `--log`, `--notify-file`, `--output-file` arguments, and `ValueHint::CommandWithArguments` to `<command...>` trailing args (verification: Zsh/Fish completion scripts contain file-path hints)
- [ ] Task 10: Add integration tests in `tests/integration.rs` verifying: (a) `completions bash/zsh/fish/powershell` produce non-empty stdout with exit 0, (b) `completions invalid` exits with code 2, (c) `--state` rejects invalid values with exit 2 (verification: `cargo test --test integration` passes)
- [ ] Task 11: Run `prek run -a` and fix any fmt/clippy/test failures (verification: `prek run -a` exits 0)

## Future Work

- Dynamic job-ID completion via custom completer (separate proposal)
- Nushell / Elvish support
- Automated completion installation (e.g. `completions install` subcommand)
