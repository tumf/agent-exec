# Design: per-Change versioning without automatic release

## Lifecycle boundary

Conflux invokes the repository `on_merged` hook after each Change merge. That callback is the versioning unit. The repository must not add session-level batching or deduplication because the requested invariant is one patch increment per Change.

## Version-only target

The existing `bump-patch` target is an explicit release preparation command. `cargo release patch --execute --no-confirm --no-publish` still performs later release steps such as tag creation and push unless separately disabled.

The automatic target therefore needs all three independent safeguards:

```text
--no-publish --no-tag --no-push
```

`cargo release` retains its version and commit steps. This produces a normal version commit after each merged Change without creating a release tag or writing to a remote.

## Hook ordering

The hook runs:

1. version-only patch bump
2. local index refresh

Shell `&&` semantics stop index refresh if versioning fails. Conflux must surface that hook failure rather than silently treating an unversioned Change as successfully post-processed.

## Verification boundary

The regression test must not execute a real version bump in the main worktree. It verifies tracked configuration and recipe structure:

- one version-only target invocation in `on_merged`
- one `make index` invocation after it
- explicit `--no-publish`, `--no-tag`, and `--no-push`
- use of the committing `cargo release ... --execute` path
- no direct `git tag`, `git push`, `cargo publish`, or `gh release` command in the automatic path

A real dry-run may inspect command expansion only when it cannot mutate repository state.

## Owner reload

Conflux owners may cache lifecycle hooks at startup. Merging this Change changes tracked configuration but does not retroactively alter the callback held by the owner performing that merge. A fresh owner is required before the next Change to guarantee the new behavior.
