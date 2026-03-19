## Implementation Tasks

- [x] Update the Unix command launch path in `src/run.rs` so multi-argument argv invocations are passed to the shell wrapper using an `exec "$@"` handoff pattern instead of a single quoted shell string (verification: `src/run.rs` contains a distinct argv launch branch that preserves wrapper usage while replacing the shell with the target workload).
- [x] Preserve existing behavior for single-string command mode and keep notify-command delivery on the current shared wrapper path (verification: `src/run.rs` still treats `command.len() == 1` as shell-string mode, and notify-command code paths remain unchanged except for any helper refactor needed to support the new argv branch).
- [x] Add integration coverage in `tests/integration.rs` that distinguishes Unix argv-mode launch from shell-string mode and verifies argv workloads complete through the exec handoff semantics (verification: new tests in `tests/integration.rs` fail before the change and pass after it).
- [x] Extend the lingering-job regression coverage to use an argv-style reproduction or equivalent assertion that demonstrates completion tracking is aligned to the intended workload boundary after the exec handoff (verification: `tests/integration.rs` includes a regression case tied to issue `#5` behavior).
- [x] Update `openspec/specs/agent-exec-run/spec.md`, `README.md`, and any related skill/reference docs to explain the Unix shell-wrapper exec handoff for argv mode without changing string-command documentation (verification: affected docs mention argv-vs-string behavior explicitly).

## Future Work

- Consider whether Windows should gain a comparable main-workload handoff strategy in a separate proposal.
