---
change_type: implementation
priority: medium
dependencies: []
references:
  - src/main.rs
  - src/run.rs
  - tests/integration.rs
  - openspec/specs/agent-exec-run/spec.md
verifications:
  - id: piped-stdin-tests
    requirement: Piped stdin is forwarded safely without changing explicit stdin or TTY behavior
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: tests/integration.rs
    evidence: cargo test --test integration stdin
    rerun: cargo test --test integration stdin
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
  - id: rust-quality-gates
    requirement: Rust formatting, lint, and regression suite remain clean
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: prek run -a output
    rerun: prek run -a
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Forward piped stdin to run workloads

**Change Type**: implementation

## Problem / Context

`agent-exec run` can already pass input to a child process through `--stdin -`, but callers must repeat that flag even when stdin is visibly connected to a pipe or redirected file. This makes the common agent and shell pattern `producer | agent-exec run -- consumer` surprising: the input is not forwarded unless the caller knows the extra flag.

The existing stdin path already provides the required durability and safety properties by materializing bytes as `stdin.bin`, recording `meta.json.stdin_file`, enforcing `--stdin-max-bytes`, and letting the supervisor open the persisted file. The missing behavior is limited to selecting that path automatically for `run` when no explicit stdin source was requested and the invocation stdin is non-TTY.

## Proposed Solution

When `agent-exec run` has neither `--stdin` nor `--stdin-file` and its own stdin is non-TTY, read stdin to EOF using the existing bounded materialization path, persist it as `stdin.bin`, and pass that file to the child process. Preserve the existing explicit forms and make their precedence unambiguous: `--stdin` and `--stdin-file` remain authoritative and suppress implicit stdin detection.

When stdin is a TTY and no explicit stdin option is present, do not read it and preserve null child stdin. This keeps `run` non-interactive and prevents accidental blocking. `create` remains explicit-only because it defines a reusable job without necessarily starting it; callers can continue using `create --stdin -`, `--stdin`, or `--stdin-file` when persisted input is intended.

This is one atomic change because automatic source selection, precedence, bounded materialization, metadata persistence, and integration coverage form one externally visible stdin contract.

## Acceptance Criteria

- `printf 'alpha' | agent-exec run -- cat` forwards the exact piped bytes to the child without requiring `--stdin -`.
- Automatically detected input uses the existing `stdin.bin` materialization, `meta.json.stdin_file`, and `--stdin-max-bytes` behavior rather than a live pipe to the detached supervisor.
- With no explicit stdin option and a TTY stdin, `run` does not read from the terminal, does not block for input, and gives the child null stdin.
- Explicit `--stdin <VALUE>`, `--stdin -`, and `--stdin-file <PATH>` behavior remains unchanged and takes precedence over automatic detection.
- Oversized implicit input fails before job launch with the existing stable `stdin_too_large` JSON error contract and does not create a runnable partial job.
- `create` does not gain implicit stdin capture; its persisted stdin remains opt-in through existing flags.
- Integration tests prove byte preservation, metadata persistence, explicit-source precedence, TTY/no-input behavior where repository test infrastructure supports it, and bounded-input failure.

## Explicit Completion Conditions

- `src/main.rs` and `src/run.rs` resolve implicit non-TTY stdin only for `run` and route it through the same bounded materialization implementation used by `--stdin -`.
- The supervisor continues receiving a persisted stdin file path; no long-lived direct pipe from the CLI process to the detached child is introduced.
- `tests/integration.rs` contains regression cases that fail when piped stdin is discarded, when explicit stdin is overwritten, or when implicit input bypasses size enforcement.
- `cargo test --test integration stdin` passes.
- `prek run -a` passes.

## Out of Scope

- Interactive terminal forwarding, PTY allocation, or attaching to a running job.
- Adding implicit stdin capture to `create`, `start`, `restart`, `serve`, or MCP execution surfaces.
- Streaming unbounded stdin directly into the supervisor or child without materialization.
- Changing the persisted `stdin.bin` or `meta.json.stdin_file` schema.
