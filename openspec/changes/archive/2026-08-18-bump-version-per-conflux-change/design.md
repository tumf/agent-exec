# Design: per-Change versioning without automatic release

## Lifecycle boundary

Conflux invokes the repository `on_merged` hook after each Change merge. That callback is the versioning unit. The repository must not add session-level batching or deduplication because the requested invariant is one patch increment per Change.

## Version-only target

The existing `bump-patch` target is an explicit release preparation command. `cargo release patch --execute --no-confirm --no-publish` still performs later release steps such as tag creation and push unless separately disabled.

The automatic target therefore needs all three independent safeguards:

```text
--no-publish --no-tag --no-push
```

`cargo release` retains its version and commit steps. This produces a normal version commit after each merged Change without creating a release tag or writing to a remote. Disabling push also disables cargo-release's behind-remote fetch check, so the automatic path performs no network access at all.

cargo-release refuses to run in a dirty working tree, counting staged, unstaged, and untracked files. `.leann` and `.tldr` are gitignored, so `make index` artifacts do not dirty the tree, but any stray untracked file in the root repository fails the bump. That failure is acceptable and intentionally loud: it is exactly the spec's "automatic version bump fails" scenario, and it must not be papered over with `allow-dirty`.

## Hook ordering

The hook runs:

1. version-only patch bump
2. local index refresh

Shell `&&` semantics stop index refresh if versioning fails. Conflux surfaces `on_merged` failures as a typed hook failure and marks the Change failed rather than silently treating an unversioned Change as successfully post-processed; the hook only needs to exit non-zero.

Each version commit lands on the base branch mid-session. Conflux pre-syncs the base into remaining workspaces before their merges, so `Cargo.toml`/`Cargo.lock` version hunks from earlier Changes are reconciled during workspace sync, not at merge time.

## Verification boundary

The regression test must not execute a real version bump in the main worktree. It verifies tracked configuration and recipe structure:

- one version-only target invocation in `on_merged`
- one `make index` invocation after it
- explicit `--no-publish`, `--no-tag`, and `--no-push`
- use of the committing `cargo release ... --execute` path
- no direct `git tag`, `git push`, `cargo publish`, or `gh release` command in the automatic path

A real dry-run may inspect command expansion only when it cannot mutate repository state.

Real execution is still required, but it belongs in a disposable clone, not the main worktree: clone the repository into a temporary directory, remove the `origin` remote so a push is impossible by construction, and run the tracked `on_merged` string twice. Two runs prove the per-Change contract directly — two sequential patch-version commits, no tag on either, clean tree afterwards — which a single run cannot.

## Owner reload

Conflux owners may cache lifecycle hooks at startup. Merging this Change changes tracked configuration but does not retroactively alter the callback held by the owner performing that merge. A fresh owner is required before the next Change to guarantee the new behavior.
