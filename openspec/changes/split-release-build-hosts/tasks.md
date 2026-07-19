## Implementation Tasks

- [x] Restrict the GitHub release workflow to the declared Linux x86_64 artifact while retaining native smoke verification, checksum creation, and release upload. (verification: integration - source path: `.github/workflows/release.yml:25-59`; command: `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/release.yml`)
- [x] Add a `mini` macOS release script that validates a version tag, builds the current tag, runs `--version` and a managed-command smoke check, packages the binary, creates a SHA-256 checksum, and supports explicit upload to the matching GitHub Release. (verification: integration - source paths: `scripts/release-macos.sh:39-75`, `tests/integration.rs:5992-6012`; command: `git tag v0.2.25-test HEAD && scripts/release-macos.sh --tag v0.2.25-test --dist-dir /var/folders/dg/xh2k12k51yb300kdz4xmtr7m0000gn/T/opencode/agent-exec-release && git tag -d v0.2.25-test`)
- [x] Make the local macOS script fail before upload for invalid tags and failed verification, without creating or mutating a GitHub Release. (verification: integration - source path: `scripts/release-macos.sh:39-49,61-72`; command: `! scripts/release-macos.sh --tag invalid-tag --dist-dir /var/folders/dg/xh2k12k51yb300kdz4xmtr7m0000gn/T/opencode/agent-exec-invalid`)
- [x] Update README installation guidance to distinguish Linux GitHub CI artifacts, local macOS artifacts, source/crates.io fallback, and unsupported Windows binaries. (verification: manual - source paths: `README.md:36-77`, `.github/workflows/release.yml:25-64`; command: `python3 -c 'from pathlib import Path; readme=Path("README.md").read_text(); workflow=Path(".github/workflows/release.yml").read_text(); assert "Linux x86_64" in readme; assert "Windows release binaries are not provided." in readme; assert "ubuntu-latest" in workflow; assert "x86_64-unknown-linux-gnu" in workflow; assert "macos" not in workflow.lower(); assert "windows" not in workflow.lower()'`)
- [x] Record repository-verifiable evidence and run all project quality gates. (verification: integration - source path: `prek.toml:12-39`; commands: `prek run -a`, `git diff --check`)

## Future Work

- Upload a macOS artifact to the next explicitly authorized public release from `mini`.
- Add Homebrew after the local macOS artifact path is stable.

## Final Validation

Expected archive gate: `cflx openspec validate split-release-build-hosts --archive-gate`

## Acceptance Notes

Archive-gate evidence is recorded in completed task verification notes and final validation results.

## Acceptance Notes

Repository-verifiable source paths and rerunnable commands are recorded in each completed implementation task above.

## Acceptance #3 Failure Follow-up
- [x] Generate the macOS checksum from inside the distribution directory so it records the archive basename, and verify the copied download artifacts with `shasum -a 256 -c`. (verification: integration - source paths: `scripts/release-macos.sh:67-71`, `tests/integration.rs:6015-6086`; command: `cargo test --test integration release_macos_checksum_verifies_after_download -- --ignored`)
