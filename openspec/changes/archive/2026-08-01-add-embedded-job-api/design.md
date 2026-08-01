# Design: Embedded Managed-Job API

## Decision: Keep a Detached Supervisor Process

A linked library can remove the public CLI subprocesses used for control operations, but it cannot safely replace the detached supervisor with a thread. The supervisor owns workload stdout/stderr, timeout escalation, notifications, state updates, and process-tree cleanup after the original caller returns or exits. Therefore embedded run still performs one process spawn: a private supervisor re-execution, not a public CLI command per operation.

## Startup Delegation

The embedding binary calls a public delegation function before its normal CLI/parser setup. The function claims an invocation only when `argv[1]` is the exact private supervisor marker. Once claimed, it strictly parses the generated supervisor arguments, including the job ID, explicit root, full-log path, optional execution settings, and the workload after `--`:

- no exact marker at `argv[1]`: return `NotSupervisor` without consuming or changing arguments;
- exact marker with a complete valid argument set: run the supervisor to completion and return/exit through a documented result;
- exact marker with missing, duplicate, malformed, or trailing unexpected arguments: fail closed without entering consumer command handling.

The marker is a reserved dispatch token, not an authentication boundary. The delegated process validates the job ID and explicit root against the pre-created job metadata before acknowledging startup. The marker and argument grammar remain private and are not a stable end-user CLI.

The typed client defaults its supervisor executable to `current_exe()`, which is valid only when the consumer installs this delegation. This runtime-enforced trade-off keeps the constructor small: a five-second startup acknowledgement detects missing delegation, and the embedding documentation places the delegation call before all consumer argument parsing. Tests and unusual packaging may supply another trusted executable explicitly; no builder or type-state API is added solely to prove delegation at compile time.

## Typed Surface

Expose one client bound to an explicit jobs root. Methods use ordinary Rust types and return domain results:

- `run`: job ID, state, initial bounded output, ranges, totals, log paths, and optional terminal data;
- `status`: managed state and exit/timing fields;
- `tail`: bounded tail, ranges, totals, encoding, and log paths;
- `list`: job summaries, truncation, and skipped count with explicit cwd/all/tag filters;
- `kill`: accepted signal and observed terminal information.

The API does not return `serde_json::Value`, CLI `Response<T>` envelopes, printed stdout, or process exit codes. Adapters convert domain errors into existing JSON/MCP/HTTP shapes.

## Supervisor Startup Acknowledgement

The launcher pre-creates metadata and logs but SHALL NOT persist `running` before delegated supervision proves it has claimed the job. After spawning the selected executable, the launcher waits up to five seconds for the supervisor to atomically create the initial `running` state with its own PID. The supervisor writes that state only after validating the reserved invocation, opening the pre-created job, and completing platform setup required before workload launch, including the Windows Job Object setup path.

The existing inline observation wait begins only after this startup acknowledgement, so its user-selected duration and `--no-wait` semantics remain unchanged. The fixed five-second acknowledgement is a launch-integrity check, not workload observation, and therefore also applies to no-wait launches.

## Launch Failure Atomicity

The current flow creates job metadata and logs before spawning supervision. Embedded launch must not report success if process spawn fails, the selected executable exits without delegation, delegated validation fails, or the five-second startup acknowledgement expires. The launcher records a terminal `failed` state for the pre-created job, with no `running` transition, then returns the structured launch error. The failed record remains available to status and list observers; the launcher does not delete the job directory or send completion notifications for a workload that never started.

The required state sequence is `created metadata -> running` after acknowledgement, or `created metadata -> failed` on launch failure. A late supervisor that attempts acknowledgement after the launcher has committed `failed` must fail closed and must not overwrite that terminal state or launch the workload.

## Error Mapping

The embedded error categories are stable domain classifications. CLI adapters preserve the existing observable contract: missing and ambiguous jobs retain their current error codes and exit code `1`; invalid input/state maps to the existing validation error code and exit code `1`; supervisor launch failures use a stable launch-failure code and exit code `1`; storage/I/O and internal failures retain their existing stable code where one exists and otherwise map to `internal_error`, exit code `1`. Clap usage errors remain outside the embedded client and retain exit code `2`. Retryability is explicit on every embedded error and is copied into the CLI JSON error object.

## Compatibility

CLI behavior remains the compatibility authority for external users. Refactoring moves implementation below the adapters but does not change flags, defaults, JSON, schema version, persistence, output bytes, or exit codes. Existing integration tests run unchanged in addition to the linked-consumer test.
