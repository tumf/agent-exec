---
change_type: implementation
priority: high
dependencies: []
references:
  - Cargo.toml
  - .github/workflows/ci.yml
  - README.md
---

# Add reproducible release distribution

**Change Type**: implementation

## Problem / Context

`agent-exec` has crates.io package metadata and a tagged GitHub release, but the repository only documents `cargo install --path .` and has no release workflow. A new user must clone the repository and install a Rust toolchain before trying the CLI. This blocks the adoption plan's first requirement: installation from a public distribution channel followed by a verified first run.

## Proposed Solution

Add the smallest maintainable release pipeline that:

- validates that the crate can be packaged and installed from its package artifact;
- publishes version-tagged binaries for supported macOS, Linux, and Windows targets to GitHub Releases;
- keeps crates.io publication explicit and safely verifiable before any publish action;
- documents exact public installation commands and unsupported target limitations;
- verifies every generated binary with `agent-exec --version` and a short managed `run` before attaching it to a release.

Use GitHub Actions and Cargo's native packaging. Do not introduce a custom release service or a separate installer framework in this change.

## Acceptance Criteria

- A `v*` tag can produce GitHub Release archives for declared macOS, Linux, and Windows targets.
- Each release binary passes `agent-exec --version` and a short `agent-exec run -- echo ...` smoke test on its build runner before publication.
- `cargo package` succeeds from the repository and the packaged crate can be installed and smoke-tested without relying on untracked repository files.
- crates.io publication has an explicit, non-accidental trigger and fails safely when credentials or package validation are unavailable.
- README installation guidance starts with public distribution options and retains source installation as a development fallback.
- Checksums are published with release artifacts, and unsupported architectures or operating systems are stated rather than implied.

## Explicit Completion Conditions

- Repository contains a release workflow with least-privilege permissions and a target matrix that names every supported artifact.
- Workflow or local verification installs the packaged crate and exercises the installed executable.
- Release artifact smoke tests exercise real CLI behavior rather than only checking file existence.
- README commands match the actual artifact names and crate package name.
- CI-equivalent formatting, lint, tests, package verification, and release-workflow validation pass.

## Out of Scope

- GUI installers.
- A hosted/cloud job execution service.
- Kubernetes, container registry, or package-manager coverage beyond crates.io and GitHub Release artifacts.
- Homebrew tap publication unless it can be added without a separate repository or ongoing manual formula maintenance; it may follow after the release artifact contract is stable.
- Changing the CLI or JSON response contract.
