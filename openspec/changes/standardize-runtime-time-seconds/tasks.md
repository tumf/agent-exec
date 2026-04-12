## Implementation Tasks

- [x] 1. Update canonical specs in `openspec/specs/agent-exec/spec.md` and `openspec/specs/agent-exec-run/spec.md` so current runtime-time contracts use seconds for human-facing flags and remove stale `snapshot-after`-centered requirements that no longer match the implemented CLI (verification: manual/integration - spec text aligns with current clap surface and `tests/integration.rs` command set).
- [x] 2. Update clap help text and option plumbing in `src/main.rs` (and any affected runtime conversion paths in `src/run.rs`, `src/create.rs`, `src/schema.rs`) so `--timeout`, `--kill-after`, and `--progress-every` are defined and interpreted in seconds at the CLI boundary while preserving correct internal timing behavior (verification: integration/unit - targeted runtime option tests plus `--help` output expectations where appropriate).
- [x] 3. Update `README.md` and `skills/agent-exec/**` so all user-facing guidance uses second-based runtime options and no longer teaches removed `snapshot-after` usage (verification: manual - targeted search over README and skill files shows canonical second-based spellings and no live `snapshot-after` examples).
- [x] 4. Update `tests/integration.rs` to verify second-based runtime options and explicit rejection of removed snapshot-era flags on the current CLI surface (verification: integration - `cargo test --test integration` covers the relevant command paths and invalid flag behavior).
- [x] 5. Run repository verification for the CLI-contract change: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all` (or `prek run -a`) (verification: command outputs show success).

## Future Work

- Review whether external automation depends on prior millisecond spellings before release notes are finalized.
- Decide separately whether persisted/result schema field names like `elapsed_ms` should ever be renamed, since that is a broader API compatibility decision.

## Acceptance #1 Failure Follow-up

- [x] Update hidden `_supervise` clap help in `src/main.rs` so `--timeout`, `--kill-after`, and `--progress-every` are documented in seconds or are no longer exposed as human-facing help text.
- [x] Remove stale `--snapshot-after` canonical examples from `openspec/specs/agent-exec-run-logging/spec.md` and `openspec/specs/agent-exec-test-harness/spec.md` so canonical specs consistently treat the flag as removed.
- [x] Add integration coverage for `agent-exec start --snapshot-after ...` usage-error rejection (and any other removed start observation flags required by the canonical spec), or narrow the checklist/spec claim to the behavior that is actually verified.
