## Implementation Tasks

- [ ] Refactor compression into a module tree while preserving the public `crate::compress` API used by `run`, `start`, `restart`, and `tail` (verification: integration - existing compression tests in `tests/integration.rs` still compile and pass).
- [ ] Add a typed route classifier that derives command family and subcommand from `CompressionInput.command` plus output shape (verification: unit - classifier tests cover `git log`, `cargo test`, `pytest`, `rg`, `docker logs`, JSON output, repeated logs, and unknown commands).
- [ ] Extend `detected_kind` generation to support specific family strings such as `git-log`, `cargo-test`, `pytest`, `search`, `docker-logs`, and `json-structure` without removing current mode names from CLI help (verification: integration - response JSON contains specific detected kinds for representative commands).
- [ ] Centralize expansion guard so every compressor candidate is rejected when it is not smaller than the observed raw stream (verification: unit - guard tests cover stdout-only, stderr-only, and mixed-stream expansion cases).
- [ ] Add shared helper utilities for bounded summaries, line deduplication, diagnostic block extraction, table row parsing, JSON shape extraction, and command token matching (verification: unit - helpers have direct tests with representative fixtures).
- [ ] Preserve config and CLI resolution behavior for `--compress`, `--rtk`, `[compression].default`, and unsupported `auto` (verification: integration - existing config/alias/conflict tests pass unchanged).
- [ ] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Specialized compressors are implemented by dependent proposals.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-compression-routing --archive-gate`
