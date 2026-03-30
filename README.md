# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: `main.rs` defines an **`Agent`** struct (`name` and `message` strings), constructs one in `main`, and prints the agent’s message.

## Current behavior

- Running the binary prints `Hello, world!` (the configured agent’s `message`).
- Unit tests in `main.rs` cover constructing an `Agent` with `name` and `message`.

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
