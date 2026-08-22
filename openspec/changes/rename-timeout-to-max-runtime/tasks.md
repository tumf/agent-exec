## Implementation Tasks

- [ ] Rename CLI launch controls from `--timeout` to `--max-runtime` across `run`, `create`, hidden supervisor handoff, restart/start plumbing, help, and shell completions; reject `--timeout` without creating a job (verification: integration - `cargo test --test integration max_runtime`; verification-id: max-runtime-contract-tests)
- [ ] Rename MCP `run.timeout` and HTTP `/exec` `timeout` to `max_runtime`, reject old and dual spellings before launch, and keep `until` strictly non-destructive (verification: integration - `cargo test --test integration max_runtime`; verification-id: max-runtime-contract-tests)
- [ ] Rename the embedded Rust launch request field to `max_runtime_ms` and update public schema/types while preserving current signal escalation and terminal-state serialization (verification: integration - `cargo test --test integration max_runtime`; verification-id: max-runtime-contract-tests)
- [ ] Migrate persisted metadata writes to `max_runtime_ms`; accept legacy stored `timeout_ms` only when reading existing job definitions, reject conflicting dual fields, and prove legacy start/restart behavior (verification: integration - `cargo test --test integration max_runtime`; verification-id: max-runtime-contract-tests)
- [ ] Update canonical specs, README, changelog, site/docs, bundled skills, examples, and test fixtures so current guidance uses `max-runtime` / `max_runtime`, distinguishes it from `until`, and contains no live public `timeout` launch examples (verification: integration - `cargo test --test integration max_runtime`; verification-id: max-runtime-contract-tests)
- [ ] Add `max_runtime`-prefixed integration coverage for real workload termination, default unlimited runtime, non-destructive `until`, old public spelling rejection on every launch surface, new metadata output, legacy metadata read/restart, and conflicting metadata rejection (verification: integration - `cargo test --test integration max_runtime`; verification-id: max-runtime-contract-tests)

## Notes

- Preserve terminal `state="timeout"` for result compatibility; only the control input is renamed.
- Legacy compatibility is confined to persisted metadata reads. Public invocation aliases are intentionally excluded because they preserve the ambiguity that caused workload loss.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate rename-timeout-to-max-runtime --archive-gate`.

Conflux acceptance must also run `make check`; staged-file hooks are not an unconditional clean-tree gate.
