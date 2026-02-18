# agent-exec

Small Rust CLI skeleton.

## Development

```bash
cargo run -- --help
cargo test
cargo fmt
cargo clippy
```

## Commands

```bash
cargo run -- greet
cargo run -- greet --name Alice
cargo run -- echo "hello"
cargo run -- version
```

## Logging

Use `RUST_LOG` to control log level:

```bash
RUST_LOG=debug cargo run --
```
