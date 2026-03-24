## Implementation Tasks

- [x] Add `#[command(version)]` to the `Cli` struct in `src/main.rs:30` (verification: `cargo run -- --version` prints version and exits 0)
- [x] Add integration test in `tests/integration.rs` asserting `--version` outputs `agent-exec <version>` and exits 0 (verification: `cargo test --test integration version`)
- [x] Run `prek run -a` to confirm fmt, clippy, and all tests pass (verification: exit 0, no warnings)

## Future Work

- Embed git commit hash or build timestamp via `build.rs` for richer version output.
