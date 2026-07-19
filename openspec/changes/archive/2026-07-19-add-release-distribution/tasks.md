## Implementation Tasks

- [x] Add `.github/workflows/release.yml` with a tag-triggered release matrix for supported macOS, Linux, and Windows targets, explicit artifact naming, and least-privilege release permissions. (verification: integration - source path: `.github/workflows/release.yml`; command: `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/release.yml`)
- [x] Package each built executable in `.github/workflows/release.yml` and generate checksums without relying on untracked files. (verification: integration - source path: `.github/workflows/release.yml`; command: `shasum -a 256 -c agent-exec-local.tar.gz.sha256`)
- [x] Smoke-test every native release executable with `agent-exec --version` and a short managed `run` before upload. (verification: e2e - source path: `.github/workflows/release.yml`; command: `agent-exec run -- echo release-smoke`)
- [x] Add crate package verification that installs the generated crate into an isolated root and executes the same smoke test. (verification: integration - `Cargo.toml:16-25`, `.github/workflows/release.yml:73-91`; `cargo package --locked --allow-dirty`, isolated extracted-package install, and JSON smoke)
- [x] Add an explicit crates.io publication job that cannot publish on ordinary pushes or pull requests and reports missing credentials as a safe failure. (verification: manual - source paths: `.github/workflows/release.yml`, `.github/workflows/ci.yml`; command: `cargo package --locked`)
- [x] Update `README.md` with verified public commands, artifact naming, supported targets, and source-build fallback. (verification: manual - `README.md:34-86`, `.github/workflows/release.yml:52-66`; `python3 -c 'from pathlib import Path; readme=Path("README.md").read_text(); workflow=Path(".github/workflows/release.yml").read_text(); assert "<ARCHIVE>.sha256" in readme; assert "shasum -a 256 -c <ARCHIVE>.sha256" in readme; assert "\"${{ matrix.archive }}.sha256\"" in workflow'`)
- [x] Run repository quality gates after the release changes. (verification: integration - `prek.toml:12-39`, `.github/workflows/ci.yml:1-30`; `prek run -a`, `cargo package --locked --allow-dirty`, isolated extracted-package smoke, `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/release.yml`, and `cflx openspec validate add-release-distribution --strict`)

## Future Work

- Publish and maintain a Homebrew tap after the GitHub Release artifact names and checksum contract have proven stable.
- Perform the first real crates.io publish and version-tag release with repository-owner credentials.
- Verify installation on clean third-party machines as tracked by the adoption-validation bead.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-release-distribution --archive-gate`

## Acceptance #1 Failure Follow-up
- [x] Align `README.md` checksum instructions with the per-archive `<ARCHIVE>.sha256` files produced by `.github/workflows/release.yml`. (verification: manual - source paths: `README.md`, `.github/workflows/release.yml`; command: `python3 -c 'from pathlib import Path; readme=Path("README.md").read_text(); workflow=Path(".github/workflows/release.yml").read_text(); assert "<ARCHIVE>.sha256" in readme; assert "shasum -a 256 -c <ARCHIVE>.sha256" in readme; assert "\"${{ matrix.archive }}.sha256\"" in workflow'`)
Repository-verifiable evidence for completed release tasks is recorded above. Archive validation remains the final gate.
- [x] Re-run all CI-equivalent quality gates and confirm the serve authentication integration test remains stable. (verification: integration - source path: `tests/serve_integration.rs`; passed commands: `prek run -a`, `cargo test --test serve_integration test_auth_token_required_returns_401 -- --nocapture`)
