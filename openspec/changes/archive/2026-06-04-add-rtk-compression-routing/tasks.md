## Implementation Tasks

- [x] Refactor compression into a module tree while preserving the public `crate::compress` API used by `run`, `start`, `restart`, and `tail` (verification: integration - existing compression tests in `tests/integration.rs` still compile and pass).
- [x] Add a typed route classifier that derives command family and subcommand from `CompressionInput.command` plus output shape (verification: unit - classifier tests cover `git log`, `cargo test`, `pytest`, `rg`, `docker logs`, JSON output, repeated logs, and unknown commands).
- [x] Extend `detected_kind` generation to support specific family strings such as `git-log`, `cargo-test`, `pytest`, `search`, `docker-logs`, and `json-structure` without removing current mode names from CLI help (verification: integration - `tests/integration.rs` compression response JSON assertions cover specific detected kinds for representative commands).
- [x] Centralize expansion guard so every compressor candidate is rejected when it is not smaller than the observed raw stream (verification: unit - `src/compress/guard.rs` tests cover stdout-only, stderr-only, and mixed-stream expansion cases).
- [x] Add shared helper utilities for bounded summaries, line deduplication, diagnostic block extraction, table row parsing, JSON shape extraction, and command token matching (verification: unit - `src/compress/utils.rs` tests cover representative fixtures).
- [x] Preserve config and CLI resolution behavior for `--compress`, `--rtk`, `[compression].default`, and unsupported `auto` (verification: integration - `tests/integration.rs` config/alias/conflict tests pass unchanged).
- [x] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Specialized compressors are implemented by dependent proposals.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-compression-routing --archive-gate`

## Acceptance #1 Failure Follow-up
- [x] コミット経路ブロッカーを解消: `src/run.rs` の `--progress-every` watcher が `state.json` を `std::fs::write` で直接上書きしており、並行する `run`/`status`/`wait` が空または途中状態の `state.json` を読んで `EOF while parsing a value at line 1 column 0` になる競合を修正した。watcher の進捗更新を `JobDir::write_state` 経由の atomic write に統一した。検証: `agent-exec run -- cargo test --test integration create_with_stdin_dash_materializes_input_for_later_start -- --nocapture` job `87983aacfe7db9648786e0fc6350bf2d` exit_code=0、`agent-exec run -- cargo test --test integration run_progress_every_updates_state -- --nocapture` job `f8df0c2442e8762dc1c5f3a5d14e8069` exit_code=0。
- [x] archive commitability を再検証 (verification: manual - `agent-exec run -- cargo fmt --all -- --check` job `68e2103c819b691889e4ae9bbfa5d069` exit_code=0、`agent-exec run -- cargo clippy --all-targets --all-features -- -D warnings` job `7972e7086eb203198c81e07d84521b91` exit_code=0、`agent-exec run -- cargo test --all` job `484b2b11b2f341daa0bb7b7fc8c66c94` exit_code=0)。
