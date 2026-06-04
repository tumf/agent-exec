## Implementation Tasks

- [ ] Add route detection for system/list/search/read/log/json/env command families (verification: unit - classifier maps `ls`, `tree`, `find`, `rg`, `grep`, `cat`, `tail`, `jq`, and `env` examples to expected kinds).
- [ ] Implement directory/list compression that groups paths by directory, preserves important filenames, caps long lists, and reports omitted counts (verification: unit - `ls/tree/find` fixtures compact into grouped summaries).
- [ ] Implement search-result compression that groups by file, reports match counts, and keeps bounded representative lines with line numbers when present (verification: unit - `rg/grep` fixtures compact by file and retain representative matches).
- [ ] Implement observed file/text compression for `cat`/`head`/`tail`-like outputs with bounded head/tail and optional code-shape summarization when language markers are visible (verification: unit - long text/code fixtures produce smaller summaries without losing first/last context).
- [ ] Implement log compression with adjacent and normalized duplicate grouping, progress-noise removal, and error-priority excerpts (verification: unit - repeated timestamped log fixtures deduplicate while preserving error lines).
- [ ] Improve JSON compression for large objects, arrays, and NDJSON streams by reporting keys, types, array lengths, and representative shape without large values (verification: unit - JSON object/array/NDJSON fixtures produce structural summaries smaller than raw).
- [ ] Implement env-like output compression that masks secret-like values and groups by prefix (verification: unit - env fixture masks keys matching secret/token/password patterns).
- [ ] Ensure all system/log/json compressors use expansion guard and preserve raw fields (verification: integration - representative commands show raw canonical stdout and guarded or smaller compression output).
- [ ] Run repository verification commands and fix regressions (verification: manual - `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`).

## Future Work

- External content fetching and cloud/container table parsing are covered separately.

## Final Validation

Expected archive gate: `cflx openspec validate add-rtk-system-log-json-compression --archive-gate`
