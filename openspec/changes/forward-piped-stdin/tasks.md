## Implementation Tasks

- [x] Resolve absent `run` stdin options to an implicit non-TTY stdin source while leaving TTY stdin and all `create` invocations explicit-only; keep `--stdin` and `--stdin-file` authoritative (verification: integration - `cargo test --test integration stdin`; verification-id: piped-stdin-tests).
- [x] Route implicit stdin through the existing bounded `stdin.bin` materialization and metadata path before supervisor launch, including the existing `stdin_too_large` error contract and cleanup semantics (verification: integration - `cargo test --test integration stdin`; verification-id: piped-stdin-tests).
- [x] Add integration regressions in `tests/integration.rs` for exact piped-byte forwarding, `meta.json.stdin_file`, explicit-source precedence, implicit size-limit failure, and null/TTY behavior where supported by the harness (verification: integration - `cargo test --test integration stdin` executes the compiled CLI and asserts child output plus persisted job evidence in `tests/integration.rs`; verification-id: piped-stdin-tests).
- [x] Update CLI help and maintained user guidance that describes stdin so the shorthand `producer | agent-exec run -- consumer`, explicit-source precedence, TTY behavior, and `create` exclusion are discoverable (verification: integration - `cargo test --test integration stdin` asserts help text sourced from `src/main.rs`, and repository review confirms updated stdin examples in maintained guidance; verification-id: piped-stdin-tests).
- [ ] Run repository formatting, lint, and regression gates without weakening hooks or warnings (verification: integration - `prek run -a` executes the tracked hooks in `prek.toml`; verification-id: rust-quality-gates).

## Future Work

- Interactive stdin attachment and PTY support require a separate lifecycle and transport design.
- Other execution surfaces may adopt implicit stdin only through separate proposals with transport-specific safety analysis.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate forward-piped-stdin --archive-gate`
