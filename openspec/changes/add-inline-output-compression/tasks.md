## Implementation Tasks

- [ ] Add compression mode parsing and config support for `off|route|errors|tests|logs|git|json|summary`. Completion condition: `src/config.rs` deserializes `[compression].default`, rejects invalid config values with a stable structured error, and exposes a resolved built-in default of `route` when no config is present. verification: unit - config parser tests cover missing config, valid `off`, valid `route`, and invalid values.

- [ ] Add `--compress <mode>` and `--rtk <mode>` to `run`, `start`, `restart`, and `tail`. Completion condition: `src/main.rs` accepts both flags for the four commands, rejects invalid CLI modes through clap usage errors, and rejects simultaneous conflicting modes with exit code `2`. verification: integration - `tests/integration.rs` invokes each command with representative `--compress`/`--rtk` flags and asserts success or usage error as appropriate.

- [ ] Resolve effective compression mode with CLI > config > built-in `route` precedence. Completion condition: command execution paths pass a resolved compression mode into observation/response construction, and `--compress off` or `--rtk off` overrides config defaults. verification: integration - tests create a temporary config with `[compression].default = "off"`, assert default omission of `compression`, then assert CLI `--compress route` restores it.

- [ ] Add response schema support for compressed observations without changing raw observation fields. Completion condition: `src/schema.rs` defines a `compression` payload with mode, applied status, detected kind, compressed stdout/stderr, original/compressed byte counts, omitted flag, and strategy list, using `skip_serializing_if` so `off` omits the field. verification: integration - tests assert raw `stdout`/`stderr`, range fields, total byte fields, and `encoding` remain present and semantically raw when `compression` is present.

- [ ] Implement built-in compression logic without invoking external `rtk`. Completion condition: repository source includes local routing and compression routines for `route`, `errors`, `tests`, `logs`, `git`, `json`, and `summary`; no runtime path shells out to `rtk`. verification: integration - tests run commands whose outputs contain repeated log lines, explicit errors, test-like failures, and JSON content, and assert `compression.applied` plus compressed payload content changes in mode-specific ways.

- [ ] Wire compression into `run`, `start`, `restart`, and `tail` response construction. Completion condition: all four commands include `compression` when resolved mode is not `off`, omit it when resolved mode is `off`, and preserve the existing JSON-only stdout envelope. verification: integration - command-specific tests parse the single stdout JSON object and assert the expected `compression` presence or absence.

- [ ] Update schema/introspection and user-facing help/doc examples as needed. Completion condition: `schema` output, CLI help, README or relevant agent-exec skill references describe `--compress`, `--rtk`, config default, and `off` compatibility behavior without mentioning `auto` as a mode. verification: integration - schema/help-oriented tests assert supported mode names include `route` and exclude `auto` where machine-readable assertions already exist; manual - review README/help text for mode list consistency.

- [ ] Run final repository verification. Completion condition: formatting, linting, and test suite pass locally. (verification: integration - run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`)。

## Future Work

- Consider adding compression to `serve` HTTP tail/status surfaces after CLI behavior stabilizes.
- Consider token-savings metrics only after compression behavior is proven useful and stable.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-inline-output-compression --archive-gate`.
