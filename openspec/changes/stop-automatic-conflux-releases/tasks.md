## Implementation Tasks

- [ ] Change `hooks.on_merged` in `.cflx.jsonc` to exactly `make index`, removing `make bump-patch` and any other release-capable step while keeping the rest of the file (including comments) intact (verification: integration - `cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects` reads the tracked config; verification-id: conflux-hook-safety)
- [ ] Add a regression test `conflux_on_merged_hook_has_no_release_side_effects` in `tests/integration.rs` that reads the tracked `.cflx.jsonc` as text (the file is JSONC with comments, so extract the `on_merged` string value textually rather than adding a JSON parser dependency) and asserts: (a) the `on_merged` value is exactly `make index`; (b) the value contains none of the tokens `bump`, `release`, `tag`, `publish`, `upload`, `push`, `cargo`, `git`; (c) the output of `make -n index` (dry-run, prints the recipe without executing it) contains no `cargo release`, `cargo publish`, `git tag`, `git push`, or `gh release` invocation (verification: integration - `cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects`; verification-id: conflux-hook-safety)
- [ ] Apply the `agent-exec-distribution` spec delta strengthening the explicit-publication requirement so version bump commits, release Git tags, and pushes of release history — not only crates.io publication — require explicit release action; in the same regression test, assert the Makefile still defines the `bump-patch`, `bump-minor`, `bump-major`, and `publish` targets so explicit release commands remain available separately from the Conflux hook (verification: integration - `cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects`; the spec-delta wording itself is validated by the archive gate; verification-id: conflux-hook-safety)

## Future Work

- Existing remote tags and releases remain unchanged unless an operator starts a separate release-repair task.

## Final Validation

Archive validation is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate stop-automatic-conflux-releases --archive-gate`
