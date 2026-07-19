## Implementation Tasks

- [x] Restrict the GitHub release workflow to the declared Linux x86_64 artifact while retaining native smoke verification, checksum creation, and release upload. (verification: integration - `.github/workflows/release.yml:25-28`; `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/release.yml` passed.)
- [x] Add a `mini` macOS release script that validates a version tag, builds the current tag, runs `--version` and a managed-command smoke check, packages the binary, creates a SHA-256 checksum, and supports explicit upload to the matching GitHub Release. (verification: integration - `scripts/release-macos.sh`; `scripts/release-macos.sh --tag v0.2.25-test --dist-dir /var/folders/dg/xh2k12k51yb300kdz4xmtr7m0000gn/T/opencode/agent-exec-release` passed on mini.)
- [x] Make the local macOS script fail before upload for invalid tags and failed verification, without creating or mutating a GitHub Release. (verification: integration - `scripts/release-macos.sh:49-58,72-75`; invalid no-upload invocation exited 2 and created no output directory.)
- [x] Update README installation guidance to distinguish Linux GitHub CI artifacts, local macOS artifacts, source/crates.io fallback, and unsupported Windows binaries. (verification: manual - `README.md:36-66`; the scripted macOS smoke procedure passed.)
- [x] Record repository-verifiable evidence and run all project quality gates. (verification: integration - `prek run -a` passed; `cflx openspec validate split-release-build-hosts --strict --evidence warn` passed; `git diff --check` passed.)

## Future Work

- Upload a macOS artifact to the next explicitly authorized public release from `mini`.
- Add Homebrew after the local macOS artifact path is stable.

## Final Validation

Expected archive gate: `cflx openspec validate split-release-build-hosts --archive-gate`

## Acceptance Notes

Archive-gate evidence is recorded in completed task verification notes and final validation results.
