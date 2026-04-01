# aria-poc-2

A Rust proof-of-concept binary. The codebase is a minimal starting point: **`core`** holds the **`Environment`** port (**`print`**, **`log`**, configured **`LoggingLevel`**: `None` / `Standard` / `Verbose`, and message-only **`LogMessageLevel`**: `Standard` / `Verbose`), the **`Llm`** port (**`base_system_prompt`**, **`complete` → `LlmCompletion`** (**`reply`**, optional **`request_body_json`**), using **`ChatMessage`** transcript entries), **`Session`** / **`ActiveSession`** and role constants **`USER_ROLE`** / **`ASSISTANT_ROLE`** in **`session`**, and **`Agent`** (**`print`**, **`log`**, optional **`active_session`**, **`start_session`**, **`stop_session`**, **`receive_message(&mut self, …) -> Result<String, ReceiveMessageError>`**). Between **`start_session`** and **`stop_session`**, **`receive_message`** appends the peer turn, merges base + session system text, calls **`Llm::complete`** with the full transcript, appends the assistant reply, and **`print`**s **`{name} -> {reply}`**. Under **`cfg(test)`**, **`test_support`** in **`core/agent.rs`** provides **`InMemoryEnvironment`** (records **`print`** and filtered **`log`** output) and **`StubLlm`** for unit tests. **`infrastructure/adapters/environment`** provides **`ShellEnvironment`** (`println!` / `eprintln!`); **`infrastructure/adapters/transport`** provides **`JsonHttp`** (blocking JSON POST over HTTP); **`infrastructure/adapters/llm`** provides **`DummyLlm`** and **`ClaudeLlm`** (Anthropic Messages API via **`JsonHttp`**, **`ClaudeLlm::new(api_key, base_system_prompt)`**); **`application/factories/create_agent`** defines **`create_agent`**. `main.rs` re-exports the public surface, loads the API key with **`anthropic_api_key_from_local_file`**, builds **`ClaudeLlm`** with a base system prompt, uses **`ShellEnvironment`**, and runs **`play_knock_knock`**: a scripted knock-knock exchange that **`start_session`**s with a dedicated session prompt, sends canned user lines via **`receive_message`** (opener, **`Who's there?`**, **`{setup} who?`** where **`setup`** is parsed from the assistant reply, then **`haha`**), prints each assistant turn, then **`stop_session`**s after the parting pleasantry. Log filtering is hierarchical: **`None`** drops all logs; **`Standard`** allows only standard messages; **`Verbose`** allows standard and verbose messages.

## Current behavior

- Running the binary uses **`LoggingLevel::Standard`**: each incoming user line is logged at **`LogMessageLevel::Standard`** (**`Aria <- …`**). If the API key file is missing or invalid, **`main`** prints to stderr and exits with code **1**. If **`start_session`** fails, **`main`** prints to stderr and exits with code **1**. There is no interactive stdin loop: the program drives the joke with fixed strings, parses the setup word from the assistant’s reply after **`Who's there?`**, and exits after the assistant’s reply to **`haha`** (the closing pleasantry). Parse failures print to stderr and exit with code **1**. With **`LoggingLevel::Verbose`**, each **`receive_message`** logs the pretty-printed **Messages API request JSON body** (model, **`max_tokens`**, merged **`system`**, **`messages`**) at **`LogMessageLevel::Verbose`**—the API key is not in that body. **`ClaudeLlm`** fills **`LlmCompletion.request_body_json`**; **`Agent`** performs the log.
- Unit tests cover construction, **`print`**, **`receive_message`** (including merged system text and transcript passed to **`Llm::complete`**), session lifecycle and errors, transcript growth across two turns, incoming and verbose request-body logging, **`merge_system_prompts`** in **`core/session.rs`**, hierarchical logging in **`core/environment.rs`**, and **`parse_setup_from_assistant_reply`** in **`main.rs`** (`knock_knock_tests`).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).

## Build and run

```sh
cargo build --workspace
cargo run
```

No stdin is read for the knock-knock flow; the binary performs a fixed sequence of turns and exits after the assistant’s parting line.

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

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). Call **`anthropic_api_key_from_local_file`** from the **`anthropic-api-key-from-local-file`** workspace crate (see [`tools/src/anthropic_api_key_from_local_file.rs`](tools/src/anthropic_api_key_from_local_file.rs)), then pass the key and an optional **base** system prompt to **`ClaudeLlm::new`**. **`main`** does that and exits with a message on stderr if the key file is missing or invalid. Use **`DummyLlm::new()`** in **`main`** only when you want a stub with no key file or network call.
