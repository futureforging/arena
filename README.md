# aria-poc-2

**Peer-to-peer AI agents** in Rust: two participants—the joke teller (**display name `Agent`**, Claude) and the scripted audience (**display name `Peer`**)—each implemented as a core [`Agent`](src/core/agent.rs) value, alternating **`receive_message`** in a knock-knock demo.

| Layer | Main pieces |
| --- | --- |
| **`core`** | **`Agent`**, **`Session`**, **`Environment`**, **`Llm`** |
| **`application`** | **`create_agent`** |
| **`infrastructure`** | **`ShellEnvironment`**, **`ClaudeLlm`**, **`KnockKnockAudienceLlm`**, **`JsonHttp`**, **`DummyLlm`** |

Each turn appends the peer line, merges system prompts, calls **`Llm::complete`**, appends the agent reply, and **`print`**s **`{name} -> {reply}`**. **`main`** loads the Anthropic key and runs **`play_knock_knock`**. Logging uses hierarchical **`LoggingLevel`** (**`None`** / **`Standard`** / **`Verbose`**); see **`core/environment.rs`**.

## Current behavior

- The demo is a scripted knock-knock joke between **`Agent`** (Claude, teller) and **`Peer`** (canned lines). There is no interactive input: **`main`** sends a synthetic greeting (**`Hello.`**) to **`Agent`** so it can open with an invitation; **`Agent`**’s session prompt instructs that step. **`Peer`** (**`KnockKnockAudienceLlm`**) replies in order: **`yes`**, **`Who's there?`**, **`{word} who?`** (using the setup word parsed from the teller’s line), then **`haha`**. **`Agent`** follows a teller session prompt for the invitation, **`Knock knock.`**, setup, punchline, and a brief parting after **`haha`**. The exchange ends after that parting line.
- **`start_session`** role pairs in **`main`** store Claude’s lines under transcript **`assistant`** and canned lines under **`user`**, matching what the Anthropic Messages API expects (each completion runs after a final **`user`** message).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).

## Build and run

```sh
cargo build --workspace
cargo run
```

No stdin is read for the knock-knock flow; the binary alternates the two participants for a fixed sequence of turns and exits after **`Agent`**’s parting line.

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

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). Call **`anthropic_api_key_from_local_file`** from the **`anthropic-api-key-from-local-file`** workspace crate (see [`tools/src/anthropic_api_key_from_local_file.rs`](tools/src/anthropic_api_key_from_local_file.rs)), then pass the key and an optional **base** system prompt to **`ClaudeLlm::new`**. **`main`** does that for **`Agent`** and exits with a message on stderr if the key file is missing or invalid. **`Peer`** uses **`KnockKnockAudienceLlm`**, which does not call the network. **`DummyLlm`** is a minimal always-same-reply **`Llm`** if you swap adapters in your own entrypoint.
