## Implementation Tasks

- [x] Task 1: Add `clap_complete` dependency to `Cargo.toml` (verification: `cargo build` succeeds; `Cargo.toml` lists `clap_complete = "4"`)
- [x] Task 2: Define `Shell` enum with `ValueEnum` derive for `bash`/`zsh`/`fish`/`powershell` in `src/main.rs` (verification: `cargo check` passes) — used `clap_complete::Shell` directly which implements `ValueEnum`
- [x] Task 3: Add `Completions { shell: Shell }` variant to the `Commands` enum in `src/main.rs` with help text (verification: `agent-exec completions --help` shows usage)
- [x] Task 4: Implement `completions` handler that calls `clap_complete::generate()` with the appropriate shell generator and writes to stdout (verification: `agent-exec completions bash` outputs non-empty script)
- [x] Task 5: Convert `--state` in `list` to use `ValueEnum` enum (`created`, `running`, `exited`, `killed`, `failed`, `unknown`) instead of `Option<String>` (verification: `agent-exec list --state invalid` exits with code 2; `cargo test --all` passes) — already implemented with `value_parser = [...]` prior to this change
- [x] Task 6: Convert `--signal` in `kill` to use `PossibleValues` or constrained parser listing `TERM`, `INT`, `KILL`, `HUP`, `USR1`, `USR2` (verification: completion script contains signal names; existing tests pass)
- [x] Task 7: Convert `--output-match-type` to use `PossibleValues` (`contains`, `regex`) across `run`, `create`, `notify set` (verification: completion script contains these values) — already implemented with `value_parser = [...]` prior to this change
- [x] Task 8: Convert `--output-stream` to use `PossibleValues` (`stdout`, `stderr`, `either`) across `run`, `create`, `notify set` (verification: completion script contains these values) — already implemented with `value_parser = [...]` prior to this change
- [x] Task 9: Add `ValueHint::DirPath` to `--cwd` arguments, `ValueHint::FilePath` to `--config`, `--env-file`, `--log`, `--notify-file`, `--output-file` arguments, and `ValueHint::CommandWithArguments` to `<command...>` trailing args (verification: Zsh/Fish completion scripts contain file-path hints)
- [x] Task 10: Add integration tests in `tests/integration.rs` verifying: (a) `completions bash/zsh/fish/powershell` produce non-empty stdout with exit 0, (b) `completions invalid` exits with code 2, (c) `--state` rejects invalid values with exit 2 (verification: `cargo test --test integration` passes)
- [x] Task 11: Run `prek run -a` and fix any fmt/clippy/test failures (verification: `prek run -a` exits 0)

## Future Work

- Dynamic job-ID completion via custom completer (separate proposal)
- Nushell / Elvish support
- Automated completion installation (e.g. `completions install` subcommand)

## Acceptance #1 Failure Follow-up

- [x] Update `kill --signal` argument behavior so common signals (`TERM`, `INT`, `KILL`, `HUP`, `USR1`, `USR2`) are offered as completion candidates while arbitrary signal names are still accepted (e.g., `QUIT`) per spec — implemented `SignalValueParser` (custom `TypedValueParser`) that exposes possible values for completion without enforcing them.
- [x] Add an integration test verifying a non-listed signal value (e.g., `QUIT`) is accepted by clap and reaches command execution (expected runtime error path, not usage error exit code 2) — `kill_signal_non_listed_value_accepted_by_clap` test added and passing.
