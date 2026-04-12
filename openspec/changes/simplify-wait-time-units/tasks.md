## Implementation Tasks

- [x] 1. Update the canonical wait-time CLI contract in `openspec/specs/agent-exec-run/spec.md` so `run --wait` and `wait` define `--until` and polling in seconds, including defaults, exclusivity, and any compatibility/rejection behavior for legacy millisecond flags (verification: integration - corresponding scenarios reference `tests/integration.rs` command coverage).
- [x] 2. Adjust clap surface definitions in `src/main.rs` and runtime option plumbing in `src/run.rs` / `src/wait.rs` so human-facing wait and poll flags accept seconds while internal timing remains correct (verification: integration - command invocations covering `run --wait` and `wait`; unit/not-testable only if conversion helpers are isolated).
- [x] 3. Update `tests/integration.rs` to cover second-based `--until` / poll behavior, defaults, and any intentional rejection path for removed millisecond spellings (verification: integration - `cargo test --test integration` cases for wait/run wait semantics).
- [x] 4. Update README and any user-facing guidance to document only the canonical second-based wait/poll flags and examples (verification: manual - targeted search shows canonical spellings and second-based examples only where intended).
- [x] 5. Run repository verification for the CLI-contract change: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` (or `prek run -a`) (verification: command outputs show success).

## Future Work

- Review whether downstream scripts or automation depend on millisecond spellings before release notes and migration guidance are finalized.
- If backward compatibility needs a phased rollout, document deprecation timing in release notes rather than in the core CLI spec.
