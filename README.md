# aria-poc-2

**Peer-to-peer AI agents** in Rust. The knock-knock demo is a **peer interaction** between a **secure agent** and a **peer agent**—two participants that alternate **`receive_message`** on the core [`Agent`](src/core/agent.rs) API.

A **secure agent** is the composition implemented as [`SecureAgent`](src/infrastructure/adapters/agent/secure_agent.rs): [`ShellEnvironment`](src/infrastructure/adapters/environment/shell_environment.rs), Anthropic [`ClaudeLlm`](src/infrastructure/adapters/llm/claude_llm.rs), and fixed display name **`SecureAgent`**. This is the first concrete shape of the secure-agent idea; **later iterations will lean further into that concept.**

A **peer agent** is the other side of the exchange (display name **`Peer`**): a core [`Agent`](src/core/agent.rs) wired with the same shell environment and [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs) for scripted replies instead of Claude.

| Layer | Main pieces |
| --- | --- |
| **`core`** | **`Agent`**, **`Session`**, **`Environment`**, **`Llm`** |
| **`application`** | **`create_agent`** |
| **`infrastructure`** | **`SecureAgent`**, **`ShellEnvironment`**, **`ClaudeLlm`**, **`KnockKnockAudienceLlm`**, **`JsonHttp`**, **`DummyLlm`** |

On each turn, the incoming peer line is appended, system prompts are merged, **`Llm::complete`** runs, the reply is appended, and output is **`print`**ed as **`{name} -> {reply}`**. **`main`** loads the Anthropic key for the secure agent and runs **`play_knock_knock`**. Logging uses hierarchical **`LoggingLevel`** (**`None`** / **`Standard`** / **`Verbose`**); see [`src/core/environment.rs`](src/core/environment.rs).

## Current behavior

- The demo is a scripted knock-knock joke between the **secure agent** (**`SecureAgent`**, Claude, joke teller) and the **peer agent** (**`Peer`**, canned lines). There is no interactive input: **`main`** sends a synthetic greeting (**`Hello.`**) to **`SecureAgent`** so it can open with an invitation; the teller’s session prompt instructs that step. **`Peer`** replies in order: **`yes`**, **`Who's there?`**, **`{word} who?`** (using the setup word parsed from the teller’s line), then **`haha`**. **`SecureAgent`** follows a teller session prompt for the invitation, **`Knock knock.`**, setup, punchline, and a brief parting after **`haha`**. The exchange ends after that parting line.
- **`start_session`** role pairs in **`main`** store Claude’s lines under transcript **`assistant`** and canned lines under **`user`**, matching what the Anthropic Messages API expects (each completion runs after a final **`user`** message).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).
- Optional: [just](https://github.com/casey/just) for shortcuts (`just lint`, `just test`, `just verify`, etc.; see [`justfile`](justfile) at the repo root).

## Build and run

```sh
cargo build --workspace
cargo run
```

No stdin is read for the knock-knock flow; the binary alternates secure agent and peer agent for a fixed sequence of turns and exits after the secure agent’s parting line (**`SecureAgent`**).

## Workspace

The root [`Cargo.toml`](Cargo.toml) is a **Cargo workspace**: the **`aria-poc-2`** package and **`tools`** ([`anthropic-api-key-from-local-file`](tools/Cargo.toml)). The root package is a workspace member automatically; **`members = ["tools"]`** adds the tools crate.

## Checks

With [just](https://github.com/casey/just) installed:

```sh
just lint
just test
```

Full sequence (format, lint, build, test)—same order as pre-commit in [`.cursor/rules/workflow.mdc`](.cursor/rules/workflow.mdc):

```sh
just verify
```

Equivalent raw `cargo` commands:

```sh
cargo fmt
cargo clippy --workspace
cargo build --workspace
cargo test --workspace
```

## API keys

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). Call **`anthropic_api_key_from_local_file`** from the **`anthropic-api-key-from-local-file`** workspace crate (see [`tools/src/anthropic_api_key_from_local_file.rs`](tools/src/anthropic_api_key_from_local_file.rs)), then pass the key and an optional **base** system prompt via **`SecureAgent::new`** (which uses **`ClaudeLlm::new`** internally). **`main`** does that for the secure agent and exits with a message on stderr if the key file is missing or invalid. The **peer agent** uses **`KnockKnockAudienceLlm`**, which does not call the network. **`DummyLlm`** is a minimal always-same-reply **`Llm`** if you swap adapters in your own entrypoint.
