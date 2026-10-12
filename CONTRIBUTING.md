# Contributing to agent-exec

## Development setup

Use the stable Rust toolchain selected by `rust-toolchain.toml`, with `rustfmt` and `clippy` installed. Run these commands from the repository checkout:

```bash
cargo build
cargo test --all
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

CI runs the quality checks through `prek` (see `prek.toml`). Before submitting a change, run:

```bash
prek run -a
```

See [AGENTS.md](AGENTS.md) for the repository layout and contribution conventions.

## Releases

Releases are cut from a `v*` tag. GitHub Actions builds, smoke-tests, and publishes the Linux x86_64 archive.

Build the macOS archive from a macOS checkout of the same release tag:

```bash
scripts/release-macos.sh --tag v<VERSION>
```

Add `--upload` to upload the archive and checksum to the existing GitHub Release:

```bash
scripts/release-macos.sh --tag v<VERSION> --upload
```
