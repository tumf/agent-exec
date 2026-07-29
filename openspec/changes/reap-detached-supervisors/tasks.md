## Implementation Tasks

- [ ] Add Unix-owned supervisor reaping at the shared `spawn_supervisor_process` boundary in `src/run.rs`, retaining the spawned `Child` until `wait()` completes without blocking the launch response or changing the returned supervisor PID (verification: integration - `cargo test supervisor_reaping`; verification-id: supervisor-reaping-tests).
- [ ] Preserve platform and lifecycle boundaries: keep the Windows Job Object handshake unchanged, avoid process-wide `SIGCHLD` or `waitpid(-1, ...)`, and keep all `run`, `start`, `restart`, serve, and MCP launches wired through the common spawn path (verification: integration - `cargo test supervisor_reaping` exercises the compiled process lifecycle and source review confirms no surface-specific bypass; verification-id: supervisor-reaping-tests).
- [ ] Add a Unix regression that keeps the launcher alive, runs bounded short jobs, and inspects child process state to fail when finished supervisors remain zombies; keep the default test under one second or mark it `heavy` if that cannot be made reliable (verification: integration - `cargo test supervisor_reaping`; verification-id: supervisor-reaping-tests).
- [ ] Add an MCP regression that runs repeated short jobs through one live MCP server, verifies their supervisor children are reaped, and confirms the server remains protocol-responsive without changing disconnect-does-not-cancel behavior (verification: integration - `cargo test --test mcp_integration mcp_reaps_finished_supervisors`; verification-id: mcp-lifecycle-tests).
- [ ] Run the tracked `prek.toml` formatting, lint, and regression gates without weakening hooks, warnings, platform guards, or existing process-lifecycle assertions (verification: integration - `prek run -a` executes the repository checks declared in `prek.toml`; verification-id: rust-quality-gates).

## Future Work

- Existing zombie processes require an operational restart of their still-living parents after the fixed binary is deployed; that cleanup is intentionally separate from repository implementation.
- A shared reaper service or async-runtime integration may be considered only if future process-launch sites require coordinated shutdown or bounded concurrency beyond this per-child ownership fix.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate reap-detached-supervisors --archive-gate`
