---
change_type: implementation
priority: high
dependencies: []
references:
  - .cflx.jsonc
  - Makefile
  - tests/integration.rs
  - openspec/specs/agent-exec-distribution/spec.md
verifications:
  - id: per-change-version-bump
    requirement: Every successful Conflux change merge creates exactly one local patch-version commit without creating a tag or performing push or publication
    phase: pre-integration
    owner: conflux-acceptance
    trigger: pull-request-validation
    automation: prek.toml
    evidence: focused integration test output proving the tracked hook and Makefile target preserve the per-change version-only contract
    rerun: cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release
    prerequisites: []
    execution_class: repository-local
    completion_role: change-blocking
---

# Bump patch version for every Conflux change

**Change Type**: implementation

## Problem / Context

The repository previously ran `make bump-patch && make index` after each Conflux merge. That did increment the version per change, but the existing `bump-patch` target also created a Git tag and pushed release history. Removing the target from `on_merged` stopped those unsafe release side effects, but also removed the desired per-change version increment.

The required policy is one patch-version increment for every successfully merged Conflux Change. A run processing two Changes therefore produces two sequential patch-version commits. Versioning must remain separate from tagging, pushing, crates.io publication, and GitHub Release publication.

## Proposed Solution

Add a dedicated Makefile target for an unattended, local patch-version bump. It SHALL update `Cargo.toml` and `Cargo.lock` as needed and create the version commit, while explicitly disabling tag creation, push, and package publication.

Configure `hooks.on_merged` to invoke that version-only target followed by `make index`. Conflux already invokes `on_merged` for each completed Change; the hook must remain per-callback and must not batch or deduplicate version increments across Changes.

Replace the regression test that forbids all version changes with a focused contract test. It shall prove that the tracked hook invokes exactly one version-only bump per callback, keeps index refresh, and that the bump target contains explicit no-tag, no-push, and no-publish safeguards.

## Acceptance Criteria

- Each successfully merged Conflux Change increments the package patch version exactly once.
- Two Changes merged in one Conflux session produce two sequential patch-version increments and two version commits.
- Each increment updates the package version consistently in tracked Cargo manifests and lock data.
- The automatic bump creates no Git tag.
- The automatic bump performs no Git push.
- The automatic bump performs no crates.io or GitHub Release publication.
- Local index refresh still runs after each Change's version bump.
- Explicit release targets remain available for intentional tagging and publication.

## Explicit Completion Conditions

- `.cflx.jsonc` invokes one dedicated version-only Makefile target and then `make index` in `hooks.on_merged`.
- The dedicated target uses `cargo release patch --execute --no-confirm` with explicit safeguards equivalent to `--no-publish --no-tag --no-push`.
- The dedicated target creates the version commit; it does not merely leave modified version files uncommitted.
- A focused integration test reads the tracked `.cflx.jsonc` and Makefile and fails if the hook omits the bump, invokes it more than once, omits index refresh, or allows tag, push, or publication.
- The canonical distribution specification distinguishes automatic per-Change version commits from explicit release actions.
- `cargo test --test integration conflux_on_merged_hook_bumps_patch_without_release` passes.

## Out of Scope

- Automatically creating or pushing a version tag.
- Automatically publishing to crates.io or GitHub Releases.
- Changing minor/major release commands.
- Batching several Changes into one version increment.
- Rewriting or deleting existing `v0.2.34` or `v0.2.35` history.
- Changing Conflux globally outside this repository.

## Rollout Note

A running Conflux owner may retain the hook command loaded at startup. After this Change is merged, future Conflux work must use a newly started owner before relying on the new per-Change hook behavior.
