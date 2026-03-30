# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: a `message` module exposes a string, and the program prints it to stdout.

## Current behavior

- Running the binary prints `Hello, world!` (from `message::get_message()`).
- The `message` module includes a unit test that asserts the returned string.

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).

## Build and run

```sh
cargo build
cargo run
```

## Checks

```sh
cargo fmt
cargo clippy
cargo test
```

## API keys

If you use a local API key file, name it `anthropic_api_key.txt` (ignored by git per `.gitignore`).
