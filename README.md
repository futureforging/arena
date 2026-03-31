# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: **`core`** holds the **`Environment`** port (**`print`**, **`log`**, configured **`LoggingLevel`**: `None` / `Standard` / `Verbose`, and message-only **`LogMessageLevel`**: `Standard` / `Verbose`), **`Agent`** (**`print`** and **`log`** delegate to the environment), and (under **`cfg(test)`**) an **`InMemoryEnvironment`** test double in **`core/agent.rs`**; **`infrastructure/adapters/environment`** provides **`ShellEnvironment`** (`println!` / `eprintln!`); **`application/factories/create_agent`** defines **`create_agent`**. `main.rs` re-exports the public surface, builds a **`ShellEnvironment`** with an explicit **`logging_level`**, calls **`Agent::log`** then **`Agent::print`** (no direct `println!` / `eprintln!` in `main`). Log filtering is hierarchical: **`None`** drops all logs; **`Standard`** allows only standard messages; **`Verbose`** allows standard and verbose messages.

## Current behavior

- Running the binary uses **`LoggingLevel::Standard`**: one log line to stderr (standard message only; the verbose **`agent.log`** is filtered out), then **`Hello, world!`** to stdout via **`Agent::print()`** (**`ShellEnvironment`**).
- Unit tests cover construction, `print()` delegation, and hierarchical logging in **`core/environment.rs`** (`log_message_is_allowed` and default `Environment::log`).

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
cargo build
cargo test
```

## API keys

If you use a local API key file, name it `anthropic_api_key.txt` (ignored by git per `.gitignore`).
