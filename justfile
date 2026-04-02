# Run from the repo root: `just --list`, `just verify`, etc. https://github.com/casey/just

# Show available recipes
default:
    @just --list

# Format Rust sources
fmt:
    cargo fmt

# Lint with Clippy (workspace)
lint:
    cargo clippy --workspace

# Run tests (workspace)
test:
    cargo test --workspace

# Build (workspace)
build:
    cargo build --workspace

# Full check: fmt, lint, build, test (matches `.cursor` pre-commit workflow)
verify: fmt lint build test
