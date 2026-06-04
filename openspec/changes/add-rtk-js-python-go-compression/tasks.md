## Implementation Tasks

- [x] Add route detection for JS/TS commands (`tsc`, `eslint`, `biome`, `next build`, `prettier --check`, `npm/pnpm/yarn test`, package install/list commands) (verification: unit - `src/compress/route.rs:293` classifier maps representative JS/TS commands to detected kinds).
- [x] Add route detection for Python commands (`pytest`, `ruff check`, `ruff format`, `mypy`, `pip list`, `pip outdated`, `uv pip ...`) (verification: unit - `src/compress/route.rs:287` and `src/compress/route.rs:321` classifier tests map representative Python commands to detected kinds).
- [x] Add route detection for Go commands (`go test`, `go build`, `go vet`, `golangci-lint run`) (verification: unit - `src/compress/route.rs:345` classifier maps representative Go commands to detected kinds).
- [x] Implement TypeScript/lint diagnostic grouping by file, rule/code, and severity with bounded representative messages (verification: unit - `src/compress/language.rs:533` tsc/eslint fixture produces grouped summaries and representative locations).
- [x] Implement JS/Python/Go test compression using text and existing JSON/NDJSON shapes when present (verification: unit - `src/compress/language.rs:557` pytest/go-test fixtures retain failures and aggregate passes).
- [x] Implement Python ruff/mypy grouping and pip package-list compacting (verification: unit - `src/compress/language.rs:541` ruff/mypy text and pip package fixtures compact correctly).
- [x] Implement Go build/vet/golangci-lint diagnostic grouping and go-test NDJSON event aggregation when observed output is NDJSON (verification: unit - `src/compress/language.rs:557` and `src/compress/language.rs:572` Go fixtures produce package/file/rule summaries).
- [x] Ensure all family compressors use expansion guard and preserve raw fields (verification: integration - `tests/integration.rs:7597` representative language commands include raw output and assert smaller-or-guarded compression output).
- [x] Run repository verification commands and fix regressions (verification: manual - `prek run -a` passed in agent-exec job `cb342873b1ab44d08dfcd95eb1b3f895`; explicit commands covered: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Commands requiring pre-execution flag injection for optimal structured output remain out of scope unless `agent-exec` later gains an explicit proxy mode.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-js-python-go-compression --archive-gate`

## Acceptance #1 Failure Follow-up

Resolved: archive commitability metadata was fixed by rewriting checkbox verification notes to cite repository-verifiable source and test evidence instead of treating archive-gate validation as implementation evidence. The authoritative archive gate remains documented only in the non-checkbox `## Final Validation` section; implementation evidence now cites `src/compress/route.rs`, `src/compress/language.rs`, and `tests/integration.rs`.

Resolved: commit-path pre-commit hooks had already passed via `prek run -a` (job `cb342873b1ab44d08dfcd95eb1b3f895`, exit 0), and this follow-up no longer contains self-referential OpenSpec validation checkboxes.
