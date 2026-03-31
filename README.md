# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: **`core`** holds the **`Environment`** port (**`print`**, **`log`**, configured **`LoggingLevel`**: `None` / `Standard` / `Verbose`, and message-only **`LogMessageLevel`**: `Standard` / `Verbose`), the **`Llm`** port (**`receive_message`**), **`Agent`** (**`print(text)`** and **`log`** delegate to the environment; **`receive_message`** logs the inbound text as **`{name} <- {message}`** at standard level, asks the LLM, then **`print`**s **`{name} -> {reply}`**), and (under **`cfg(test)`**) an **`InMemoryEnvironment`** test double in **`core/agent.rs`**; **`infrastructure/adapters/environment`** provides **`ShellEnvironment`** (`println!` / `eprintln!`); **`infrastructure/adapters/llm`** provides **`DummyLlm`** (stub adapter that always returns a fixed string); **`application/factories/create_agent`** defines **`create_agent`**. `main.rs` re-exports the public surface, builds a **`ShellEnvironment`** with an explicit **`logging_level`** and **`DummyLlm`**, and calls **`Agent::receive_message`** (no direct `println!` / `eprintln!` in `main`). Log filtering is hierarchical: **`None`** drops all logs; **`Standard`** allows only standard messages; **`Verbose`** allows standard and verbose messages.

## Current behavior

- Running the binary uses **`LoggingLevel::None`**: no stderr lines (all **`Environment::log`** output is filtered), and one stdout line from **`receive_message`**: **`Aria -> Message received.`** (**`DummyLlm`**; **`ShellEnvironment`**).
- Unit tests cover construction, `print` / `receive_message` delegation, and hierarchical logging in **`core/environment.rs`** (`log_message_is_allowed` and default `Environment::log`).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).

## Build and run

```sh
cargo build --workspace
cargo run
```

## Workspace

The root [`Cargo.toml`](Cargo.toml) is a **Cargo workspace**: the **`aria-poc-2`** package and **`tools`** ([`anthropic-api-key-from-local-file`](tools/Cargo.toml)). The root package is a workspace member automatically; **`members = ["tools"]`** adds the tools crate.

## Checks

```sh
cargo fmt
cargo clippy --workspace
cargo build --workspace
cargo test --workspace
```

## API keys

If you use a local API key file, name it `anthropic_api_key.txt` (ignored by git per `.gitignore`).
