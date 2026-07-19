## Implementation Tasks

- [x] Add `.github/workflows/release.yml` with a tag-triggered release matrix for supported macOS, Linux, and Windows targets, explicit artifact naming, and least-privilege release permissions. (verification: integration - passed `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.11 .github/workflows/release.yml`)
- [x] Package each built executable in `.github/workflows/release.yml` and generate checksums without relying on untracked files. (verification: integration - passed archive extraction and `shasum -a 256 -c agent-exec-local.tar.gz.sha256` for the native macOS archive)
- [x] Smoke-test every native release executable with `agent-exec --version` and a short managed `run` before upload. (verification: e2e - passed on the native macOS release binary after archive extraction; workflow parses `ok`, `state`, and `exit_code` before upload on every matrix runner)
- [x] Add crate package verification that installs the generated crate into an isolated root and executes the same smoke test. (verification: integration - passed `cargo package --locked --allow-dirty`, extracted the generated `.crate`, installed it to `mktemp -d`, then asserted the installed binary's JSON smoke response)
- [x] Add an explicit crates.io publication job that cannot publish on ordinary pushes or pull requests and reports missing credentials as a safe failure. (verification: manual - `publish-crates-io` is limited to `workflow_dispatch && inputs.publish_crates_io`, uses only `CARGO_REGISTRY_TOKEN`, and exits before `cargo publish` when it is unset)
- [x] Update `README.md` with verified public commands, artifact naming, supported targets, and source-build fallback. (verification: manual - native archive extraction, checksum verification, `--version`, and managed smoke passed from a temporary directory; crates.io command matches package name)
- [x] Run repository quality gates after the release changes. (verification: integration - passed `prek run -a`, `cargo package --locked --allow-dirty`, isolated extracted-package install and JSON smoke, `actionlint`, and `cflx openspec validate add-release-distribution --strict`)

## Future Work

- Publish and maintain a Homebrew tap after the GitHub Release artifact names and checksum contract have proven stable.
- Perform the first real crates.io publish and version-tag release with repository-owner credentials.
- Verify installation on clean third-party machines as tracked by the adoption-validation bead.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-release-distribution --archive-gate`
