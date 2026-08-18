## Implementation Tasks

- [ ] Add a dedicated Makefile target that performs one patch-version update and version commit with `cargo release`, while explicitly disabling publication, tag creation, and push; leave the existing explicit `bump-patch`, minor, major, and publication targets unchanged (verification: integration - `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release` inspects the tracked recipe and its safety flags; verification-id: per-change-version-bump)
- [ ] Configure `.cflx.jsonc` `hooks.on_merged` to invoke the dedicated version-only target exactly once followed by `make index`, preserving Conflux's one-hook-callback-per-merged-Change semantics instead of batching increments across a session (verification: integration - `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release` reads `.cflx.jsonc`, asserts one bump invocation and one index invocation in order, and rejects duplicate or release-capable commands; verification-id: per-change-version-bump)
- [ ] Replace `conflux_on_merged_hook_has_no_release_side_effects` with `conflux_on_merged_hook_bumps_patch_without_release`, covering the desired automatic version commit and the prohibited tag, push, and publication paths; update the canonical distribution requirement to distinguish automatic version commits from explicit releases (verification: integration - `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release`; verification-id: per-change-version-bump)

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate bump-version-per-conflux-change --archive-gate`.

## Future Work

- Start a fresh Conflux owner after this Change merges so subsequent Changes use the updated tracked `on_merged` command rather than a command cached by an older owner.
- Tagging, pushing, crates.io publication, and GitHub Release publication remain explicit operator actions.
