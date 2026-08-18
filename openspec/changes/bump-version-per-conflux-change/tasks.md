## Implementation Tasks

- [x] Add a dedicated Makefile target that performs one patch-version update and version commit with `cargo release`, while explicitly disabling publication, tag creation, and push; reuse the existing `command -v cargo-release` availability guard so a missing tool fails the hook with a clear message, and leave the existing explicit `bump-patch`, minor, major, and publication targets unchanged (verification: integration - `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release` inspects the tracked recipe and its safety flags; verification-id: per-change-version-bump)
- [x] Configure `.cflx.jsonc` `hooks.on_merged` to invoke the dedicated version-only target exactly once followed by `make index`, preserving Conflux's one-hook-callback-per-merged-Change semantics instead of batching increments across a session (verification: integration - `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release` reads `.cflx.jsonc`, asserts one bump invocation and one index invocation in order, and rejects duplicate or release-capable commands; verification-id: per-change-version-bump)
- [x] Replace `conflux_on_merged_hook_has_no_release_side_effects` with `conflux_on_merged_hook_bumps_patch_without_release`, covering the desired automatic version commit and the prohibited tag, push, and publication paths; update the canonical distribution requirement to distinguish automatic version commits from explicit releases (verification: integration - `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release`; verification-id: per-change-version-bump)
- [x] Fire the tracked `on_merged` command for real in a disposable clone and record the evidence in this file: clone the workspace repository into a temporary directory, remove the clone's `origin` remote so no push is possible, then run the exact `hooks.on_merged` string from the tracked `.cflx.jsonc` twice via `sh -c` to prove two sequential per-Change bumps (verification: integration - `tmp=$(mktemp -d) && git clone --quiet . "$tmp/repo" && git -C "$tmp/repo" remote remove origin`, then run the tracked hook string twice with `sh -c` inside `$tmp/repo`; evidence: `git -C "$tmp/repo" log --oneline -3` shows two consecutive `chore: Release agent-exec version` commits with sequential patch versions, `git -C "$tmp/repo" tag --points-at HEAD` and `git -C "$tmp/repo" tag --points-at HEAD~1` print nothing, `git -C "$tmp/repo" status --porcelain` is empty, and the clone has no remote; prerequisite `cargo-release` via `make setup`; if LEANN/TLDR tooling is unavailable in the clone, the version-bump half must still execute for real and the `make index` half may be evidenced with `make -n index` plus a note here; verification-id: per-change-version-bump)

## Notes

- The dedicated target is `version-bump-patch` in the `Makefile`; `.cflx.jsonc` `hooks.on_merged` is `make version-bump-patch && make index`. The explicit `bump-patch`, `bump-minor`, `bump-major`, `publish`, and `publish-tag` targets are unchanged.
- evidence: `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release` -> `test result: ok. 1 passed; 0 failed`.
- evidence: `cargo fmt --all -- --check` clean; `cargo clippy --all-targets --all-features -- -D warnings` clean; `cargo test --all` -> all suites pass.
- evidence (real per-Change execution, disposable clone with `origin` removed, tracked hook string run twice via `sh -c`):
  - `git remote -v` printed nothing before and after both runs.
  - Run 1: `Upgrading agent-exec from 0.2.35 to 0.2.36`, commit `chore: Release agent-exec version 0.2.36`, 2 files changed (`Cargo.toml`, `Cargo.lock`).
  - Run 2: `Upgrading agent-exec from 0.2.36 to 0.2.37`, commit `chore: Release agent-exec version 0.2.37`, 2 files changed.
  - `git log --oneline -3` -> `77525db chore: Release agent-exec version 0.2.37` / `c36a26c chore: Release agent-exec version 0.2.36` / `4193f5d seed: tracked hook under test`.
  - `git tag --points-at HEAD` and `git tag --points-at HEAD~1` both printed nothing.
  - `git status --porcelain` printed nothing; `Cargo.toml` ended at `version = "0.2.37"`.
  - The `make index` half executed for real in the clone: the TLDR warm cache completed (`Indexed 38 files, found 1179 edges`); `leann` is not installed on this host and the existing recipe tolerates that per-tool failure, so the hook still exited 0. The `make -n index` fallback was therefore not needed.
- The clone is created from committed history, so this change's `Makefile` and `.cflx.jsonc` were copied in and committed once inside the disposable clone (`seed: tracked hook under test`) before the two hook runs, ensuring the tracked hook under test is the one exercised.
- cargo-release 1.0.0 prints a `  Publishing agent-exec` step header even under `--no-publish`; no upload occurs. Comparison dry runs confirm it: with all steps enabled the run compiles the crate and prints `Uploading agent-exec v0.2.36` / `aborting upload due to dry run`, while the `--no-publish` run performs no compile and prints no `Uploading` line. Likewise `--no-tag`/`--no-push` remove the `Pushing ... to origin` line that appears with all steps enabled.

## Final Validation

Archive validation is the authoritative final OpenSpec gate. Expected archive gate: `cflx openspec validate bump-version-per-conflux-change --archive-gate`.

## Future Work

- Start a fresh Conflux owner after this Change merges so subsequent Changes use the updated tracked `on_merged` command rather than a command cached by an older owner. End-to-end confirmation on the next merged Change: `git log --oneline` shows exactly one `chore: Release agent-exec version` commit immediately after that Change's merge commit, `git tag --points-at <version commit>` prints nothing, and the branch's ahead-of-remote count grows without any push.
- Tagging, pushing, crates.io publication, and GitHub Release publication remain explicit operator actions.
