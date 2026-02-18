# Agent Guide (agent-exec)

For agentic coding assistants operating in this repository.

## Project Snapshot

- Language: Rust (edition 2024)
- Crate: `agent-exec`
- Binary: `agent-exec` (`src/main.rs`)
- Contract: stdout is JSON-only; diagnostic logs go to stderr (`src/main.rs`, `src/schema.rs`).
- CI: runs `prek` hooks (`prek.toml`, `.github/workflows/ci.yml`).

## Setup

- Toolchain: `stable` (pinned by `rust-toolchain.toml`).
- Required components: `rustfmt`, `clippy`.
- Optional (recommended): install `prek` for local parity with CI.

## Repository Layout

- `src/main.rs`: clap CLI surface + logging + error-to-JSON boundary.
- `src/schema.rs`: stdout JSON envelopes and persisted `meta.json` / `state.json` models.
- `src/run.rs`, `src/status.rs`, `src/tail.rs`, `src/wait.rs`, `src/kill.rs`: command implementations.
- `src/jobstore.rs`: job directory management and lookup.
- `tests/integration.rs`: contract tests that execute the compiled `agent-exec` binary.

## Local Debugging Tips

- Increase log verbosity (stderr only): `RUST_LOG=debug cargo run --bin agent-exec -- ...`
- Many tests set an isolated root via `AGENT_EXEC_ROOT`; for manual runs prefer `--root <dir>` or `AGENT_EXEC_ROOT=/tmp/agent-exec`.
- Integration tests rely on stdout being a single JSON object; avoid printing extra newlines or text.

## Build / Lint / Test Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run --bin agent-exec -- --help

# Format
cargo fmt --all
cargo fmt --all -- --check

# Lint (CI-style)
cargo clippy --all-targets --all-features -- -D warnings

# Test
cargo test --all
cargo test --all -- --nocapture
```

## Run a Single Test

Rust test filtering is substring-based and works for unit + integration tests.

```bash
# Unit test (in src/**)
cargo test my_test_name
cargo test some_module::my_test_name

# Integration tests live in tests/integration.rs
cargo test --test integration
cargo test --test integration run_returns_json_with_job_id
cargo test --test integration run_returns_json_with_job_id -- --nocapture
```

## Prek (matches CI)

CI runs `prek` as the source of truth.

```bash
prek run -a
prek run cargo-fmt -a
prek run cargo-clippy -a
prek run cargo-test -a
```

## Tooling Rules Discovered

- Cursor rules: none found in `.cursor/rules/` or `.cursorrules`.
- Copilot rules: none found in `.github/copilot-instructions.md`.
- Treat `prek.toml` as policy: Rust changes should pass fmt, clippy `-D warnings`, and tests.

## Code Style Guidelines

### Formatting / Diffs

- Use `rustfmt` defaults; do not hand-align or reflow unrelated code.
- Prefer small, focused diffs; avoid drive-by formatting.

### Imports

- Prefer explicit imports; avoid glob imports (`use foo::*`) unless strongly justified.
- Group imports as: (1) `std` (2) external crates (3) local crate/modules.
- In library code prefer `crate::...`; in the binary/tests prefer `agent_exec::...`.

### Types / Ownership

- Prefer borrowing (`&str`, `&Path`); allocate (`String`, `PathBuf`) only when needed.
- Use `u64` for millisecond durations (matches CLI flags and tests).
- Serialization contract matters:
  - Public stdout JSON may omit absent optional fields via `skip_serializing_if`.
  - Persisted `state.json` must keep some option keys present as `null` (see `src/schema.rs`).

### Naming Conventions

- Rust: files/modules `snake_case`; types `CamelCase`; vars/functions `snake_case`; consts `SCREAMING_SNAKE_CASE`.
- JSON schema: fields are `snake_case`; keep the envelope stable (`schema_version`, `ok`, `type`).

### Error Handling / Exit Codes

- stdout is JSON-only: successful commands print exactly one JSON object.
- stderr is for diagnostics only (use `tracing`; do not emit JSON envelopes there).
- Use `anyhow::Result<T>` internally; convert to stable API errors at the CLI boundary:
  - use stable `error.code` values (e.g., `job_not_found`)
  - always include `error.retryable` (see `ErrorResponse` in `src/schema.rs`)
- Exit codes (tests enforce): `0` success, `1` expected failure with JSON error, `2` clap/usage errors.

### Logging

- Use `tracing` macros and honor `RUST_LOG`.
- Never log secrets or sensitive env var values.

### Security / Secrets

- Do not persist real secret values.
- Masking is part of the contract: keys in `--mask` must show values as `***` in stdout JSON and `meta.json`.
- If you touch masking, logs, or JSON shapes, update `tests/integration.rs` accordingly.

## Common Pitfalls

- Do not print anything but the JSON envelope to stdout (even harmless debug text breaks tests).
- Keep JSON field names stable (`stdout_tail`/`stderr_tail`, `schema_version`, `type`); integration tests assert these.
- Be deliberate about `Option<T>` serialization: omitted vs `null` differs between stdout responses and persisted `state.json`.
- Avoid leaking secrets into logs, JSON, `meta.json`, or snapshots; tests check for this.
- Prefer adding/adjusting integration tests when changing behavior (they document the contract).

## Changing the CLI Contract

If you change flags, JSON shapes, exit codes, or persistence formats:

- Update `src/main.rs` (clap surface) and `src/schema.rs` (serde rules/types).
- Add/adjust integration tests in `tests/integration.rs`.
- Run `prek run -a` (CI parity).
