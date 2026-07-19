## Implementation Tasks

- [ ] Restrict the GitHub release workflow to the declared Linux x86_64 artifact while retaining native smoke verification, checksum creation, and release upload. (verification: integration - `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/release.yml` and inspect that no macOS or Windows runner/target remains)
- [ ] Add a `mini` macOS release script that validates a version tag, builds the current tag, runs `--version` and a managed-command smoke check, packages the binary, creates a SHA-256 checksum, and supports explicit upload to the matching GitHub Release. (verification: integration - run the script in no-upload mode for the current version and verify the archive, checksum, and smoke JSON)
- [ ] Make the local macOS script fail before upload for invalid tags and failed verification, without creating or mutating a GitHub Release. (verification: integration - invoke the script with an invalid tag in no-upload mode and assert non-zero exit)
- [ ] Update README installation guidance to distinguish Linux GitHub CI artifacts, local macOS artifacts, source/crates.io fallback, and unsupported Windows binaries. (verification: manual - inspect `README.md` and execute locally applicable checksum, extraction, version, and managed-command instructions)
- [ ] Record repository-verifiable evidence and run all project quality gates. (verification: integration - `prek run -a`, `cflx openspec validate split-release-build-hosts --strict --evidence warn`, and `git diff --check`)

## Future Work

- Upload a macOS artifact to the next explicitly authorized public release from `mini`.
- Add Homebrew after the local macOS artifact path is stable.

## Final Validation

Expected archive gate: `cflx openspec validate split-release-build-hosts --archive-gate`
