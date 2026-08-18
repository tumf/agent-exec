---
change_type: implementation
priority: high
dependencies: []
references:
  - .cflx.jsonc
  - Makefile
  - openspec/specs/agent-exec-distribution/spec.md
verifications:
  - id: conflux-hook-safety
    requirement: Ordinary Conflux merges do not mutate versions, create tags, publish, or push
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: configuration assertion test output including a make -n dry-run recipe scan of the retained index hook
    rerun: cargo test --test integration conflux_on_merged_hook_has_no_release_side_effects
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Stop automatic Conflux releases

**Change Type**: implementation

## Problem / Context

The repository configures Conflux `on_merged` as `make bump-patch && make index`. `make bump-patch` runs `cargo release ... --execute`, which commits a version bump and creates a Git tag. The release tool can also push according to repository release configuration. Therefore every ordinary Conflux change can mutate release history after merge, collide with an existing remote tag, and attempt an unauthorized public write.

This occurred after `add-mcp-completion-notifications`: the hook created local `v0.2.32` release history and attempted to push `main` and the tag even though the remote had advanced and already contained `v0.2.32`.

## Proposed Solution

Make ordinary Conflux completion non-releasing. Retain only the local index refresh in `on_merged`; version bumps, tags, pushes, registry publication, and GitHub Release upload remain explicit operator actions through their existing release commands.

Add a deterministic configuration regression test that parses `.cflx.jsonc` and fails if the ordinary merge hook references version bump, release, tag, publish, or push commands. Keep release targets themselves unchanged.

## Acceptance Criteria

- Completing an ordinary Conflux change does not modify `Cargo.toml` or `Cargo.lock` versions.
- Completing an ordinary Conflux change does not create a commit or Git tag.
- Completing an ordinary Conflux change does not push, publish, upload, or contact a release endpoint.
- The retained post-merge action may refresh local indexes only.
- Explicit Makefile release and publication commands remain available and require separate human invocation.
- A repository-local test prevents release-capable commands from returning to `hooks.on_merged`.

## Explicit Completion Conditions

- `.cflx.jsonc` sets `hooks.on_merged` to exactly `make index` and contains no release-capable command in `on_merged`.
- A focused test reads the actual tracked config and proves the prohibited command classes are absent.
- The retained hook target is itself non-releasing: the test scans the `make -n index` dry-run recipe (which prints commands without executing them) and finds no `cargo release`, `cargo publish`, `git tag`, `git push`, or `gh release` invocation. The hook is not executed for real during verification.
- Explicit Makefile release targets (`bump-patch`, `bump-minor`, `bump-major`, `publish`) still exist, satisfying the criterion that release commands remain available as separate operator actions.
- Canonical distribution requirements state that version/tag creation and publication require explicit release action, not an ordinary Conflux merge.

## Out of Scope

- Changing the versioning scheme or release targets.
- Publishing, pushing, deleting remote tags, or rewriting existing release history.
- Changing Conflux globally outside this repository.
- Removing local index generation.
