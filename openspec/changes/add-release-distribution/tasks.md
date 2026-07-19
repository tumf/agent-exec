## Implementation Tasks

- [ ] Add `.github/workflows/release.yml` with a tag-triggered release matrix for supported macOS, Linux, and Windows targets, explicit artifact naming, and least-privilege release permissions. (verification: integration - command: `actionlint .github/workflows/release.yml`)
- [ ] Package each built executable in `.github/workflows/release.yml` and generate checksums without relying on untracked files. (verification: integration - command: extract each generated archive and run `shasum -a 256 -c SHA256SUMS` or the Windows equivalent)
- [ ] Smoke-test every native release executable with `agent-exec --version` and a short managed `run` before upload. (verification: e2e - command: parse `agent-exec run -- echo release-smoke` stdout with a JSON assertion on `ok`, `state`, and `exit_code` in `.github/workflows/release.yml`)
- [ ] Add crate package verification that installs the generated crate into an isolated root and executes the same smoke test. (verification: integration - command: `cargo package` followed by `cargo install --path target/package/agent-exec-* --root <temp>` and `<temp>/bin/agent-exec run -- echo package-smoke`)
- [ ] Add an explicit crates.io publication job that cannot publish on ordinary pushes or pull requests and reports missing credentials as a safe failure. (verification: manual - file: `.github/workflows/release.yml`; inspect trigger, job condition, permissions, secret references, and a package-only run with no registry token)
- [ ] Update `README.md` with verified public commands, artifact naming, supported targets, and source-build fallback. (verification: manual - file: `README.md`; execute each locally applicable installation command from a clean temporary directory)
- [ ] Run repository quality gates after the release changes. (verification: integration - command: `prek run -a && cargo package` plus the isolated package smoke command)

## Future Work

- Publish and maintain a Homebrew tap after the GitHub Release artifact names and checksum contract have proven stable.
- Perform the first real crates.io publish and version-tag release with repository-owner credentials.
- Verify installation on clean third-party machines as tracked by the adoption-validation bead.

## Final Validation

Archive validation itself is the authoritative final OpenSpec validation gate.
Expected archive gate: `cflx openspec validate add-release-distribution --archive-gate`
