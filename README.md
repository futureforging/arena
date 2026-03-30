# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: **`core`** holds the **`Environment`** port, **`Agent`**, and (under **`cfg(test)`**) an **`InMemoryEnvironment`** test double in **`core/agent.rs`**; **`infrastructure/adapters/environment`** provides **`ShellEnvironment`** (via `println!`); **`application/factories/create_agent`** defines **`create_agent`**. `main.rs` re-exports the public surface and runs an agent whose **`Agent::print`** sends the message through the environment. `main` builds an agent with `ShellEnvironment` and calls `print()` (no direct `println!` in `main`).

## Current behavior

- Running the binary prints `Hello, world!` via the shell environment when `Agent::print()` runs.
- Unit tests cover construction and that `print()` forwards the message to the environment.

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
