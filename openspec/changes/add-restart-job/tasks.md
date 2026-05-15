## Implementation Tasks

- [x] Add the `restart` CLI surface in `src/main.rs` and route it from the command dispatcher.
  Completion condition: `agent-exec restart --help` documents `JOB_ID`, `--signal`, `--wait`, `--until`, `--forever`, `--no-wait`, and `--max-bytes`, and `restart` uses JSON-only command handling like other job commands.
  verification: integration - add coverage in `tests/integration.rs` that invokes `agent-exec restart --help` or clap usage paths without breaking existing command parsing.

- [x] Implement restart execution using the persisted job definition from `meta.json` without generating a new job id.
  Completion condition: restart opens the existing `JobDir`, reads `meta.json`, resolves shell wrapper/env/stdin/notification/runtime settings consistently with `start`, and calls supervisor launch with `params.job_id == existing job_id`.
  verification: integration - add `tests/integration.rs` coverage that restarts a terminal job and asserts the response `job_id` equals the original id and no additional job directory appears under the test root.

- [x] Terminate running jobs before relaunching them.
  Completion condition: when state is `running`, restart sends the requested signal to the current process tree, waits for terminal observation, and does not spawn the replacement until the old process is no longer reported as running.
  verification: integration - add `tests/integration.rs` coverage that runs a long-lived command, restarts it, and asserts the original process is no longer alive while the replacement command produces fresh observable output.

- [x] Treat `created` jobs as start-equivalent restart targets.
  Completion condition: restart on a `created` job launches the persisted command and returns `type="restart"` with the same observation fields as `start`.
  verification: integration - add `tests/integration.rs` coverage that creates a job, restarts it with `--wait`, and asserts stdout contains the created command's output and the job id is unchanged.

- [x] Reset per-run artifacts before the fresh supervisor launch.
  Completion condition: restart truncates `stdout.log`, `stderr.log`, and `full.log`, and removes or replaces stale completion state so subsequent `tail`, `status`, and inline observation represent the new run.
  verification: integration - add `tests/integration.rs` coverage that runs a job with distinguishable pre/post output, restarts it, and asserts `tail` and inline output do not include stale pre-restart bytes.

- [x] Preserve persisted metadata and masking semantics during restart.
  Completion condition: restart does not rewrite `meta.json` except for already-supported persisted updates, keeps masked response values redacted, and uses runtime env values for `create`/`start` lifecycle jobs.
  verification: integration - add `tests/integration.rs` coverage that creates a job with env/mask/stdin metadata, restarts it, and asserts output uses the real runtime value while JSON response remains masked where applicable.

- [x] Return `RunData`-compatible restart responses with inline observation controls.
  Completion condition: restart responses include `job_id`, `state`, `tags`, `stdout_log_path`, `stderr_log_path`, `elapsed_ms`, `waited_ms`, `stdout`, `stderr`, range metrics, byte totals, `encoding`, and terminal result fields when applicable.
  verification: integration - add `tests/integration.rs` coverage for restart with default wait and `--no-wait`, asserting response fields match the existing run/start contract.

- [x] Preserve auto-GC and error behavior expectations.
  Completion condition: successful restart performs bounded auto-GC with the same best-effort semantics as `run`/`start`, and missing/ambiguous/invalid job ids continue to emit stable JSON errors without non-JSON stdout.
  verification: integration - add `tests/integration.rs` coverage that restarts a valid job with an old terminal job present and asserts target remains inspectable; add missing-id coverage asserting `ok=false` JSON with stable error code.

- [x] Update user-facing documentation and command schema surfaces that enumerate subcommands.
  Completion condition: README/help/schema-related documentation includes `restart` alongside `run/status/tail/wait/kill/list/start/create` where command lists are maintained.
  verification: manual - inspect `README.md`, `src/main.rs`, and schema/help output to confirm restart is represented without snapshot-era fields.

- [x] Run Rust quality gates.
  Completion condition: formatting, linting, and tests pass locally.
  verification: integration - run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

## Future Work

- Add rotated per-attempt log history if users later need historical logs for the same job id.
- Add a public restart counter or attempt id only if a later use case requires it.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-restart-job --archive-gate`
