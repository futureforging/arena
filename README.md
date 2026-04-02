# aria-poc-2

**Peer-to-peer AI agents** in Rust: two **`Agent`** peers (**User** scripted, **Assistant** via Claude) alternate **`receive_message`** in a knock-knock demo.

| Layer | Main pieces |
| --- | --- |
| **`core`** | **`Agent`**, **`Session`**, **`Environment`**, **`Llm`** |
| **`application`** | **`create_agent`** |
| **`infrastructure`** | **`ShellEnvironment`**, **`ClaudeLlm`**, **`KnockKnockUserLlm`**, **`JsonHttp`**, **`DummyLlm`** |

Each turn appends the peer line, merges system prompts, calls **`Llm::complete`**, appends the agent reply, and **`print`**s **`{name} -> {reply}`**. **`main`** loads the Anthropic key and runs **`play_knock_knock`**. Logging uses hierarchical **`LoggingLevel`** (**`None`** / **`Standard`** / **`Verbose`**); see **`core/environment.rs`**.

## Current behavior

- The demo is a scripted knock-knock joke between **`Assistant`** and **`User`**. There is no interactive input: **`main`** sends the opening line to the assistant, then the two agents take turns. The assistant is expected to answer **`Knock knock.`** to the opener; the **User** agent (via **`KnockKnockUserLlm`**) replies **`Who's there?`**, then **`{setup} who?`** using the setup word parsed from the assistant’s line after **`Who's there?`**, then **`haha`**. The assistant follows a session prompt that matches that script. The exchange stops after the assistant’s reply to **`haha`** (a short parting line).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).

## Build and run

```sh
cargo build --workspace
cargo run
```

No stdin is read for the knock-knock flow; the binary alternates the two agents for a fixed sequence of turns and exits after the assistant’s parting line.

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

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). Call **`anthropic_api_key_from_local_file`** from the **`anthropic-api-key-from-local-file`** workspace crate (see [`tools/src/anthropic_api_key_from_local_file.rs`](tools/src/anthropic_api_key_from_local_file.rs)), then pass the key and an optional **base** system prompt to **`ClaudeLlm::new`**. **`main`** does that for the assistant agent and exits with a message on stderr if the key file is missing or invalid. The user agent uses **`KnockKnockUserLlm`**, which does not call the network. **`DummyLlm`** is a minimal always-same-reply **`Llm`** if you swap adapters in your own entrypoint.
