## Implementation Tasks

- [ ] Add route detection for JS/TS commands (`tsc`, `eslint`, `biome`, `next build`, `prettier --check`, `npm/pnpm/yarn test`, package install/list commands) (verification: unit - argv classifier maps representative commands to JS/TS detected kinds).
- [ ] Add route detection for Python commands (`pytest`, `ruff check`, `ruff format`, `mypy`, `pip list`, `pip outdated`, `uv pip ...`) (verification: unit - argv classifier maps representative commands to Python detected kinds).
- [ ] Add route detection for Go commands (`go test`, `go build`, `go vet`, `golangci-lint run`) (verification: unit - argv classifier maps representative commands to Go detected kinds).
- [ ] Implement TypeScript/lint diagnostic grouping by file, rule/code, and severity with bounded representative messages (verification: unit - tsc/eslint/biome fixtures produce grouped summaries and retain representative locations).
- [ ] Implement JS/Python/Go test compression using text and existing JSON/NDJSON shapes when present (verification: unit - pytest/vitest/jest/go-test fixtures retain failures and aggregate passes).
- [ ] Implement Python ruff/mypy grouping and pip package-list compacting (verification: unit - ruff JSON/text, mypy text, and pip JSON/text fixtures compact correctly).
- [ ] Implement Go build/vet/golangci-lint diagnostic grouping and go-test NDJSON event aggregation when observed output is NDJSON (verification: unit - Go fixtures produce package/file/rule summaries).
- [ ] Ensure all family compressors use expansion guard and preserve raw fields (verification: integration - representative commands include raw output and smaller or guarded compression output).
- [ ] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- Commands requiring pre-execution flag injection for optimal structured output remain out of scope unless `agent-exec` later gains an explicit proxy mode.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-js-python-go-compression --archive-gate`
