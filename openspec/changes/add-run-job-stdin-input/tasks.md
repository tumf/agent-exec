## Implementation Tasks

- [ ] Add `--stdin <VALUE>` and `--stdin-file <PATH>` to `run` and `create` in `src/main.rs`, enforce their mutual exclusion, preserve the run/create definition-time option alignment rule, and reserve `--stdin -` for caller-stdin passthrough (verification: integration - `tests/integration.rs` covers clap rejection for dual specification and success paths for both subcommands).
- [ ] Extend persisted metadata in `src/schema.rs` and job creation helpers so jobs can store an optional `stdin_file` definition pointing at materialized stdin content in the job directory (verification: unit/integration - `meta.json` written by both `run` and `create` contains the same `stdin_file` shape when stdin is defined).
- [ ] Implement stdin materialization helpers in `src/run.rs` and shared create/run setup code so `--stdin -`, inline `--stdin <STRING>`, and `--stdin-file <PATH>` all write `stdin.bin` before supervisor launch, while tty-backed `--stdin -` fails with stable API error `stdin_required` (verification: integration - heredoc, pipe, inline string, file copy, and tty failure cases in `tests/integration.rs`).
- [ ] Update `src/start.rs` and supervisor launch paths in `src/run.rs` so `start` and `run` open persisted `stdin_file` content for child stdin when present and keep `Stdio::null()` behavior when absent (verification: integration - `create --stdin ...` followed by `start --wait` reproduces the saved input, and no-stdin jobs keep current behavior).
- [ ] Document the new stdin contract in `README.md` and the relevant OpenSpec deltas, including the non-goals around interactive stdin and masking, then run strict proposal validation (verification: manual - README examples show heredoc/pipe/file usage; strict - `python3 /Users/tumf/.agents/skills/cflx-proposal/scripts/cflx.py validate add-run-job-stdin-input --strict`).

## Future Work

- Consider whether a follow-up should expose non-sensitive stdin metadata such as `stdin_present` or `stdin_bytes` in JSON responses.
- Consider whether large inline `--stdin <STRING>` payloads need explicit size guidance or a dedicated `--stdin-file` recommendation in CLI help.
