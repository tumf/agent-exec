## Implementation Tasks

- [ ] Remove version bumping from `.cflx.jsonc` ordinary `on_merged` processing while retaining only non-release local index refresh (verification: integration - `cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects` reads the tracked config; verification-id: conflux-hook-safety)
- [ ] Add a deterministic repository-local regression test that reads `.cflx.jsonc` and rejects version bump, release, tag, publish, upload, and push commands in `hooks.on_merged` (verification: integration - `cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects`; verification-id: conflux-hook-safety)
- [ ] Strengthen the canonical explicit-publication requirement so version bump commits and Git tags, not only crates.io publication, require explicit release action (verification: integration - `cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects` asserts Makefile release targets remain separate from `.cflx.jsonc`; verification-id: conflux-hook-safety)

## Future Work

- Existing remote tags and releases remain unchanged unless an operator starts a separate release-repair task.

## Final Validation

Archive validation is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate stop-automatic-conflux-releases --archive-gate`
