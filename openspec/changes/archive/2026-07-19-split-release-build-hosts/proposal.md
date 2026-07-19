---
change_type: implementation
priority: high
dependencies: []
references:
  - openspec/specs/agent-exec-distribution/spec.md
  - .github/workflows/release.yml
  - README.md
---

# Split release builds between GitHub Linux and local macOS

**Change Type**: implementation

## Problem / Context

The current release workflow builds macOS, Linux, and Windows artifacts in GitHub Actions. Windows is not a supported distribution target. macOS artifacts must be built on the trusted local `mini` host rather than GitHub-hosted runners. Only Linux shall be built in GitHub Actions.

## Proposed Solution

- Restrict the GitHub Actions release build to Linux x86_64.
- Add a repository script that builds, smoke-tests, packages, checksums, and uploads the macOS artifact from `mini` to an existing GitHub Release.
- Document Linux CI distribution, local macOS distribution, and explicit Windows non-support.
- Keep crates.io publication explicit and independent of ordinary pushes and pull requests.

## Acceptance Criteria

- A release tag builds and uploads only the Linux x86_64 archive and checksum through GitHub Actions.
- No GitHub-hosted macOS or Windows release build exists.
- On `mini`, one documented command builds the macOS release binary, exercises `--version` and a managed command, creates a versioned archive and checksum, and uploads both to the matching GitHub Release.
- The local macOS command refuses a missing/non-version tag and does not upload when build or smoke verification fails.
- README states that Windows release binaries are not provided.

## Explicit Completion Conditions

- `.github/workflows/release.yml` has a Linux-only build job and retains checksum verification before GitHub Release creation.
- A checked-in executable script provides the local macOS release path and supports a no-upload verification mode.
- The script is run on `mini` in no-upload mode against the current version.
- `prek run -a`, strict OpenSpec validation, and repository script checks pass.

## Out of Scope

- Windows binaries or Windows support work.
- GitHub-hosted macOS builds.
- Homebrew packaging.
- Automatic remote triggering of the local `mini` build.
- Publishing a new external release during proposal implementation unless separately authorized.
