# Design: align-create-run-definition-options

## Overview

The lifecycle split creates two different user entrypoints for job definition:

- `create`: persist a job definition without executing it
- `run`: define a job and start it immediately

To keep the CLI predictable, both commands should share one definition-time option model. The difference between them should be execution timing, not what kind of durable metadata they can express.

## Option Boundary

The proposal formalizes two categories of options.

### Definition-time options

These contribute to persisted job metadata (`meta.json`) and MUST be accepted by both `create` and `run`:

| Option | Description |
|--------|-------------|
| command argv | The command and its arguments |
| `--cwd` | Working directory |
| `--env KEY=VALUE` | Environment variable assignments (may be repeated) |
| `--env-file FILE` | Environment variable file paths (may be repeated) |
| `--inherit-env` / `--no-inherit-env` | Whether to inherit the caller's environment |
| `--mask KEY` | Keys whose values are masked in output |
| `--timeout` | Timeout in milliseconds |
| `--kill-after` | Milliseconds after SIGTERM before SIGKILL |
| `--progress-every` | State refresh interval in milliseconds |
| `--shell-wrapper` | Shell wrapper program and flags |
| `--tag TAG` | Job tags (may be repeated; duplicates deduplicated) |
| `--notify-command COMMAND` | Shell command for job-completion notification sink |
| `--notify-file PATH` | File path for NDJSON completion notification sink |
| `--output-pattern PATTERN` | Pattern to match against output lines |
| `--output-match-type TYPE` | Match type: `contains` or `regex` |
| `--output-stream STREAM` | Stream selector: `stdout`, `stderr`, or `either` |
| `--output-command COMMAND` | Shell command for output-match notification sink |
| `--output-file PATH` | File path for NDJSON output-match notification sink |

### Launch / observation-time options

These control how a caller observes or waits for execution and do NOT belong on `create`. They are accepted only by `run` and `start`:

| Option | Description |
|--------|-------------|
| `--snapshot-after MS` | Milliseconds to wait before returning snapshot |
| `--tail-lines N` | Number of tail lines to include in snapshot |
| `--max-bytes N` | Maximum bytes for tail |
| `--wait` | Wait for terminal state before returning |
| `--wait-poll-ms MS` | Poll interval while waiting |
| `--log PATH` | Override full.log path (run-only) |

## Contract Shape

The simplest durable rule is:

1. `create` accepts all definition-time options and persists them.
2. `run` accepts the same definition-time options and persists them through the same underlying creation path.
3. `run` may additionally accept immediate-start and observation options.
4. `start` consumes the persisted definition rather than redefining it.

This keeps future evolution straightforward: when a new field belongs in `meta.json`, it should be added to both `create` and `run` unless there is an explicit documented reason not to.

## First Concrete Application

This proposal also applies the general rule immediately to the metadata families already under discussion:

- `--tag`
- completion notification options such as `--notify-command` / `--notify-file`
- output-match notification options such as `--output-pattern`, sink selection, match mode, and stream selection

For these fields, the intended contract is:

1. `create` accepts and persists them.
2. `create` does not execute notification sinks or perform output matching.
3. `run` accepts the same definition-time inputs and persists the same metadata shape.
4. `start` activates whatever was saved by `create`.
5. Later changes continue to flow through metadata mutation commands such as `tag set` and `notify set`.

## Future Alignment Rule for Implementors

When adding a new persisted metadata field to `meta.json` in the future:

1. **Classify the option first.** Decide whether it is definition-time (goes into `meta.json`) or launch/observation-time (does not).
2. **Wire through both paths.** If definition-time, add the CLI flag and persisted field to **both** `create` and `run` in `src/main.rs`, `src/create.rs`, and `src/run.rs`. A one-sided addition is a spec violation.
3. **Keep `start` consuming, not redefining.** `start` reads `meta.json` and launches the job; it must not require the caller to re-specify definition-time options.
4. **Test both paths.** Add integration tests in `tests/integration.rs` that verify jobs created via `create` and via `run` produce equivalent `meta.json` for the new field.

The canonical places to stay aligned are:
- `src/main.rs` — CLI argument definitions for `Create` and `Run` variants
- `src/create.rs` — `CreateOpts` struct and `execute()` metadata persistence
- `src/run.rs` — `RunOpts` struct and `execute()` metadata persistence
- `tests/integration.rs` — alignment verification tests

## Relationship to Existing Active Changes

- `add-create-start-lifecycle` defines the lifecycle split and shared primitives.
- `add-job-tags` and `extend-notify-set-output-matches` define specific metadata families that should follow the shared rule.

This proposal absorbs the narrower tags/notifications-only alignment intent into a single broader rule so future definition-time options do not need separate one-off policy proposals.
