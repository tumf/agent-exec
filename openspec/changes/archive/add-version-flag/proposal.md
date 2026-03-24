# Add `--version` Flag to CLI

## Problem / Context

`agent-exec` currently has no way for users or scripts to query the binary's
version.  The `Cli` struct (`src/main.rs`) uses clap's `#[derive(Parser)]` but
does not include `#[command(version)]`, so neither `--version` nor `-V` is
recognized.

This is a standard expectation for any published CLI tool and is required for
troubleshooting, CI pinning, and compatibility checks.

## Proposed Solution

Add `#[command(version)]` to the `Cli` struct.  clap will automatically read
the version string from `Cargo.toml` (`env!("CARGO_PKG_VERSION")`) and print
it when `--version` or `-V` is passed.

No changes to JSON output, exit codes, or the schema contract are needed.
`--version` prints plain text to stdout and exits with code `0`, which is the
standard clap behavior.

## Acceptance Criteria

1. `agent-exec --version` prints `agent-exec <version>` and exits `0`.
2. `agent-exec -V` behaves identically.
3. The version string matches the value in `Cargo.toml`.
4. No regression in existing integration tests.
5. An integration test verifies the `--version` output.

## Out of Scope

- Embedding git commit hash or build timestamp (can be a separate follow-up).
- Adding a `version` subcommand (the flag is sufficient).
