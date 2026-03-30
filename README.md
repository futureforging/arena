# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: `main.rs` defines an **`EnvironmentAdapter`** trait (e.g. **`ShellAdapter`** using `println!`), an **`Agent`** (`name`, `message`, and an adapter), **`create_agent`**, and **`Agent::print`**, which sends the message through the adapter. `main` builds an agent with `ShellAdapter` and calls `print()` (no direct `println!` in `main`).

## Current behavior

- Running the binary prints `Hello, world!` via the shell adapter when `Agent::print()` runs.
- Unit tests cover construction and that `print()` forwards the message to the adapter.

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
