# Design: detached supervisor reaping

## Current Process Ownership

`spawn_supervisor_process` creates an `_supervise` child, records its PID in initial job state, and returns control to the execution surface. The supervisor owns the workload and persists terminal state. This separation is the basis of managed-job detachment.

The launcher nevertheless remains the operating-system parent of the supervisor while both are alive. On Unix, when the supervisor exits before a long-lived launcher such as `agent-exec mcp`, its exit status must be collected by that launcher. Dropping `std::process::Child` does not perform that collection.

## Decision

Transfer each Unix supervisor `Child` to a minimal background reaping operation that owns exactly that child and calls `wait()`. Return the already captured PID and `started_at` through the current API without joining the reaper.

The implementation should use the Rust standard library and add no runtime dependency. The reaper is resource cleanup only: terminal job state remains the supervisor's responsibility, and reaper failure must not rewrite job state or emit protocol output.

## Why Per-Child Ownership

A process-wide `SIGCHLD` handler or `waitpid(-1, ...)` would affect every child process in the host process, including children created by dependencies. Per-child ownership keeps status collection local to the `Command::spawn()` call that created the supervisor and avoids cross-library races.

Keeping all behavior at `spawn_supervisor_process` also covers CLI, start/restart, HTTP serve, and MCP consistently. MCP-specific cleanup would leave other long-lived callers vulnerable and duplicate lifecycle logic.

## Detachment Semantics

The reaping operation must not signal the supervisor, hold protocol streams, or make the caller wait for supervisor completion. If a short-lived CLI exits first, the operating system reparents the still-running supervisor as before. If a long-lived MCP or HTTP server remains alive, it eventually collects the supervisor's exit status.

No external response or persisted representation changes. The supervisor PID remains available for state initialization and signaling, and the Windows handshake continues to use the spawned process exactly as before.

## Verification Strategy

A meaningful regression must keep the parent process alive after multiple short supervisors finish. Merely checking terminal job state is insufficient because the current implementation already writes correct terminal state while leaking zombie process entries.

Unix integration coverage should therefore inspect the live launcher's direct children and fail if a finished `_supervise` child remains in `Z` state. The check must use bounded polling to tolerate scheduling, clean up its child processes, and remain below one second. If reliable process-table verification exceeds one second on supported CI hosts, mark that test `heavy` and retain a sub-second lower-level ownership test in the default suite.

The MCP test additionally sends another protocol request after jobs finish, proving reaping does not block the server thread or corrupt stdio JSON-RPC output.

## Risks

- One OS thread per concurrently running supervisor may become expensive at very high concurrency. This proposal accepts that ceiling because it is the smallest standard-library fix and current requirements do not call for a global runtime. A coordinated reaper can replace it if measured concurrency makes thread cost material.
- Process-table assertions vary across Unix implementations. Tests must use `cfg(unix)`, bounded polling, and the repository's existing compiled-binary harness rather than assuming Linux-only `/proc` paths.
- Reaper diagnostics must stay on stderr through existing tracing and must not introduce stdout output.
