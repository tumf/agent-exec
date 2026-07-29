## Implementation Tasks

- [x] Add Unix-owned supervisor reaping at the shared `spawn_supervisor_process` boundary in `src/run.rs`, retaining the spawned `Child` until `wait()` completes without blocking the launch response or changing the returned supervisor PID (verification: integration - `cargo test supervisor_reaping`; verification-id: supervisor-reaping-tests).
- [x] Preserve platform and lifecycle boundaries: keep the Windows Job Object handshake unchanged, avoid process-wide `SIGCHLD` or `waitpid(-1, ...)`, and keep all `run`, `start`, `restart`, serve, and MCP launches wired through the common spawn path (verification: integration - `cargo test supervisor_reaping` exercises the compiled process lifecycle and source review confirms no surface-specific bypass; verification-id: supervisor-reaping-tests).
- [x] Add a Unix regression that keeps the launcher alive, runs bounded short jobs, and inspects child process state to fail when finished supervisors remain zombies; keep the default test under one second or mark it `heavy` if that cannot be made reliable (verification: integration - `cargo test supervisor_reaping`; verification-id: supervisor-reaping-tests).
- [x] Add an MCP regression that runs repeated short jobs through one live MCP server, verifies their supervisor children are reaped, and confirms the server remains protocol-responsive without changing disconnect-does-not-cancel behavior (verification: integration - `cargo test --test mcp_integration mcp_reaps_finished_supervisors`; verification-id: mcp-lifecycle-tests).
- [x] Run the tracked `prek.toml` formatting, lint, and regression gates without weakening hooks, warnings, platform guards, or existing process-lifecycle assertions (verification: integration - `prek run -a` executes the repository checks declared in `prek.toml`; verification-id: rust-quality-gates).

## Notes

- Implementation: `src/run.rs` adds `reap_spawned_child`, and `spawn_supervisor_process` transfers the spawned supervisor `Child` to it after `init_state` and the Windows handshake. The Unix path owns exactly that `Child` on a dedicated thread that calls `wait()`; the non-Unix path is a no-op, so the Job Object handshake and Windows process contract are untouched. No `SIGCHLD` handler and no `waitpid(-1, ...)` is installed.
- Wiring: `_supervise` is spawned from exactly one place (`src/run.rs:461`), and `run` (`src/run.rs:693`), `start` (`src/start.rs:80`), `restart` (`src/restart.rs:82`), and serve (`src/serve.rs:390`) all call `spawn_supervisor_process`, so MCP and HTTP serve inherit reaping with no surface-specific cleanup.
- evidence: `cargo test supervisor_reaping` → 2 passed in 0.06s (`supervisor_reaping_collects_owned_child_exit_status`, `supervisor_reaping_neither_blocks_the_launcher_nor_signals_the_child`).
- evidence: `cargo test --test mcp_integration mcp_reaps_finished_supervisors` → 1 passed in 0.87s, under the one-second policy, so no `heavy` marker is needed.
- evidence: both regressions were confirmed to fail against the pre-fix behavior. With the ownership transfer replaced by `drop(supervisor)`, the MCP test failed with `MCP server 77067 still owns unreaped supervisor children: [77136, 77137, 77139]`; with `reap_spawned_child` stubbed to a no-op, the ownership test failed with `owned child 81639 was not reaped; stat=Some("ZN")`.
- evidence: `prek run -a` → trailing-whitespace, end-of-file, check-toml, check-yaml, `cargo fmt --check`, `cargo clippy -D warnings`, and `cargo test --all` all Passed.
- Test-harness addition: `tests/support/mod.rs` gains a `#[cfg(unix)] proc_table` module (`process_stat`, `is_zombie`, `zombie_children`, `poll_until`) that reads `ps` instead of `/proc`, so the assertions work on macOS and Linux and stay bounded.
- Observed once during a full-suite run and not reproducible: `compression_expansion_guard_applies_per_stream` failed on the stderr-content assertion under parallel load. It is a pre-existing timing race in `observe_inline_output`, which breaks as soon as either stream has bytes; that code is untouched by this change, and the subsequent full `prek run -a` runs were green.

## Future Work

- Existing zombie processes require an operational restart of their still-living parents after the fixed binary is deployed; that cleanup is intentionally separate from repository implementation.
- A shared reaper service or async-runtime integration may be considered only if future process-launch sites require coordinated shutdown or bounded concurrency beyond this per-child ownership fix.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate reap-detached-supervisors --archive-gate`
