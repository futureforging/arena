# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: **`core`** holds the **`Environment`** port (**`print`**, **`log`**, configured **`LoggingLevel`**: `None` / `Standard` / `Verbose`, and message-only **`LogMessageLevel`**: `Standard` / `Verbose`), the **`Llm`** port (**`receive_message`**), **`Agent`** (**`print(text)`** and **`log`** delegate to the environment; **`receive_message`** logs the inbound text as **`{name} <- {message}`** at standard level, asks the LLM, then **`print`**s **`{name} -> {reply}`**), and (under **`cfg(test)`**) an **`InMemoryEnvironment`** test double in **`core/agent.rs`**; **`infrastructure/adapters/environment`** provides **`ShellEnvironment`** (`println!` / `eprintln!`); **`infrastructure/adapters/llm`** provides **`DummyLlm`** (stub, **`DummyLlm::new()`**) and **`ClaudeLlm`** (Anthropic Messages API via **`reqwest`**, **`ClaudeLlm::new(api_key, system_prompt)`**); **`application/factories/create_agent`** defines **`create_agent`**. `main.rs` re-exports the public surface, loads the API key with **`anthropic_api_key_from_local_file`** from the workspace tools crate, builds **`ClaudeLlm`** with that key and a **`SYSTEM_PROMPT`**, uses a **`ShellEnvironment`** with an explicit **`logging_level`**, and calls **`Agent::receive_message`** (or prints an error to stderr and exits if the key file is missing or invalid; no direct `println!` / `eprintln!` in `main` on success). Log filtering is hierarchical: **`None`** drops all logs; **`Standard`** allows only standard messages; **`Verbose`** allows standard and verbose messages.

## Current behavior

- Running the binary uses **`LoggingLevel::None`**: no stderr lines on success (all **`Environment::log`** output is filtered). If the API key file is missing or invalid, **`main`** prints to stderr and exits with code **1**. Otherwise one stdout line from **`receive_message`**: **`Aria -> …`** with the model’s reply (**`ClaudeLlm`**; **`ShellEnvironment`**). With **`LoggingLevel::Verbose`**, stderr also shows a pretty-printed static snapshot from **`ClaudeLlm::static_config_json`** (model, **`max_tokens`**, optional **`system`** only—not the per-request **`messages`** body).
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

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). Call **`anthropic_api_key_from_local_file`** from the **`anthropic-api-key-from-local-file`** workspace crate (see [`tools/src/anthropic_api_key_from_local_file.rs`](tools/src/anthropic_api_key_from_local_file.rs)), then pass the key and an optional system prompt to **`ClaudeLlm::new`**. **`main`** does that and exits with a message on stderr if the key file is missing or invalid. Use **`DummyLlm::new()`** in **`main`** only when you want a stub with no key file or network call.
