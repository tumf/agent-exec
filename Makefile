# Makefile for agent-shell

.DEFAULT_GOAL := build

.PHONY: build help install release run test clean fmt fmt-check lint check setup index pre-commit pre-commit-hooks ci bump-patch bump-minor bump-major publish publish-tag

CARGO ?= cargo
BIN ?= agent-exec
ARGS ?=

# Default target - build debug version
build:
	@echo "Building debug version..."
	$(CARGO) build

# Help message
help:
	@echo "Available targets:"
	@echo "  make (default)         - Build debug version"
	@echo "  make build             - Build debug version"
	@echo "  make install           - Install the binary to ~/.cargo/bin"
	@echo "  make release           - Build optimized release version"
	@echo "  make run               - Run $(BIN) (use ARGS=...)"
	@echo "  make test              - Run all tests"
	@echo "  make clean             - Clean build artifacts"
	@echo "  make fmt               - Format code with rustfmt"
	@echo "  make fmt-check          - Check formatting (no changes)"
	@echo "  make lint              - Run clippy linter (CI style)"
	@echo "  make check             - Run fmt-check, lint, and test"
	@echo "  make index             - Build Serena symbol index (.serena/cache)"
	@echo "  make setup             - Setup development environment"
	@echo "  make pre-commit        - Run prek on all files (matches CI)"
	@echo "  make pre-commit-hooks  - Install git pre-commit hooks (prek)"
	@echo "  make ci                - Run CI checks (fmt-check, lint, test)"
	@echo "  make bump-patch        - Bump patch version and tag (no publish)"
	@echo "  make bump-minor        - Bump minor version and tag (no publish)"
	@echo "  make bump-major        - Bump major version and tag (no publish)"
	@echo "  make publish           - Publish current version to crates.io"
	@echo "  make publish-tag       - Publish specific git tag to crates.io"

# Install binary to ~/.cargo/bin
install:
	@echo "Installing $(BIN)..."
	$(CARGO) install --path . --bin $(BIN)
	@echo "Installation complete. Binary installed to ~/.cargo/bin/$(BIN)"

# Build release version
release:
	@echo "Building release version..."
	$(CARGO) build --release
	@echo "Release binary: target/release/$(BIN)"

# Run binary
run:
	$(CARGO) run --bin $(BIN) -- $(ARGS)

# Run tests
test:
	@echo "Running tests..."
	$(CARGO) test --all

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	$(CARGO) clean

# Format code
fmt:
	@echo "Formatting code..."
	$(CARGO) fmt --all

# Check formatting (no changes)
fmt-check:
	@echo "Checking formatting..."
	$(CARGO) fmt --all -- --check

# Run linter
lint:
	@echo "Running clippy..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings

# Run all checks (format check, lint, test)
check: fmt-check lint test
	@echo "All checks passed!"

# Build Serena symbol index under .serena/cache
index:
	@echo "Indexing project for Serena..."
	@command -v serena >/dev/null 2>&1 || (echo "serena CLI not found. Install it first (e.g. 'uv tool install serena')." && exit 1)
	serena project index . --log-level INFO
	@echo "Serena index complete."

# Setup development environment
setup: pre-commit-hooks
	@echo "Setting up development environment..."
	@command -v rustfmt >/dev/null 2>&1 || rustup component add rustfmt
	@command -v clippy >/dev/null 2>&1 || rustup component add clippy
	@command -v cargo-release >/dev/null 2>&1 || $(CARGO) install cargo-release
	@echo "Development environment setup complete!"

# Run prek checks locally (matches CI)
pre-commit:
	@set -e; \
	if command -v prek >/dev/null 2>&1; then PREK=prek; \
	elif [ -x "$$HOME/.local/bin/prek" ]; then PREK="$$HOME/.local/bin/prek"; \
	else \
		echo "prek not found. Run 'make pre-commit-hooks' to install it."; \
		exit 1; \
	fi; \
	"$$PREK" run -a

ci: check

# Install pre-commit hooks
pre-commit-hooks:
	@set -e; \
	echo "Installing pre-commit hooks (prek)..."; \
	if command -v prek >/dev/null 2>&1; then PREK=prek; \
	elif [ -x "$$HOME/.local/bin/prek" ]; then PREK="$$HOME/.local/bin/prek"; \
	else \
		echo "prek not found. Installing to $$HOME/.local/bin..."; \
		mkdir -p "$$HOME/.local/bin"; \
		curl -LsSf https://github.com/j178/prek/releases/latest/download/prek-installer.sh | sh; \
		PREK="$$HOME/.local/bin/prek"; \
	fi; \
	"$$PREK" install --overwrite --hook-type pre-commit; \
	echo "Pre-commit hook installed. Run 'make pre-commit' to verify."

# Bump patch version (0.1.0 -> 0.1.1) and create git tag (no publish)
bump-patch:
	@echo "Bumping patch version..."
	@command -v cargo-release >/dev/null 2>&1 || (echo "cargo-release not found. Run 'make setup' first." && exit 1)
	@cargo release patch --execute --no-confirm --no-publish
	@echo "Patch version bumped and tagged successfully"
	@echo "To publish to crates.io, run: make publish"

# Bump minor version (0.1.0 -> 0.2.0) and create git tag (no publish)
bump-minor:
	@echo "Bumping minor version..."
	@command -v cargo-release >/dev/null 2>&1 || (echo "cargo-release not found. Run 'make setup' first." && exit 1)
	@cargo release minor --execute --no-confirm --no-publish
	@echo "Minor version bumped and tagged successfully"
	@echo "To publish to crates.io, run: make publish"

# Bump major version (0.1.0 -> 1.0.0) and create git tag (no publish)
bump-major:
	@echo "Bumping major version..."
	@command -v cargo-release >/dev/null 2>&1 || (echo "cargo-release not found. Run 'make setup' first." && exit 1)
	@cargo release major --execute --no-confirm --no-publish
	@echo "Major version bumped and tagged successfully"
	@echo "To publish to crates.io, run: make publish"

# Publish current version to crates.io
publish:
	@echo "Publishing to crates.io..."
	@CURRENT_VERSION=$$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2); \
	echo "Current version: $$CURRENT_VERSION"; \
	read -p "Proceed with publish? [y/N] " confirm && [ "$$confirm" = "y" ] || exit 1; \
	$(CARGO) publish
	@echo "Published successfully!"

# Publish specific git tag to crates.io
publish-tag:
	@echo "Available tags:"; \
	git tag -l | tail -10; \
	read -p "Enter tag to publish (e.g., v0.1.34): " tag; \
	if [ -z "$$tag" ]; then \
		echo "Error: No tag specified"; \
		exit 1; \
	fi; \
	CURRENT_BRANCH=$$(git rev-parse --abbrev-ref HEAD); \
	echo "Checking out tag: $$tag"; \
	git checkout $$tag && \
	$(CARGO) publish && \
	git checkout $$CURRENT_BRANCH && \
	echo "Published $$tag successfully and returned to $$CURRENT_BRANCH"
