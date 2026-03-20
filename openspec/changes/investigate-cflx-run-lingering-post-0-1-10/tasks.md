## Implementation Tasks

- [ ] Record the reproduced `cflx run` evidence in repository-facing proposal/design text, including the observed command shape, success-like tail output, persistent `running` state, and lingering `_supervise` + `cflx run` processes (verification: proposal/design files contain the reproduced evidence and differentiate it from the already-fixed stdio-only case).
- [ ] Add or adapt regression coverage in `tests/integration.rs` so issue `#5` is no longer represented only by synthetic shell reproductions; the test path must model a workload that appears complete in logs while `status` incorrectly remains `running` (verification: a new or updated test in `tests/integration.rs` documents the post-`0.1.10` failure shape).
- [ ] Audit the `agent-exec` state model around `src/run.rs`, `src/status.rs`, and `src/wait.rs` to define what evidence is required before `running` can be trusted once success-like completion output is already present (verification: design/spec text names the specific code paths and the distinction between process liveness and log completion).
- [ ] Decide and document whether the remaining fix-forward work belongs in `agent-exec`, `cflx`, or coordinated changes across both, based on the reproduced `cflx run` lifecycle (verification: proposal/design text contains an explicit ownership conclusion or a bounded set of next-step hypotheses).

## Future Work

- If the final root cause is upstream in `cflx`, create a dedicated follow-up change proposal in that project and link it from this investigation.
