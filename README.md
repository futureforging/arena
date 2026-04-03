# aria-poc-2

**Peer-to-peer AI agents** in Rust. The knock-knock demo is a **peer interaction** between a **secure agent** and a **peer agent**—two participants that alternate **`receive_message`** on the core [`Agent`](src/core/agent.rs) API.

A **secure agent** is the composition implemented as [`SecureAgent`](src/infrastructure/adapters/agent/secure_agent.rs): [`ShellEnvironment`](src/infrastructure/adapters/environment/shell_environment.rs), Anthropic [`ClaudeLlm`](src/infrastructure/adapters/llm/claude_llm.rs), and fixed display name **`SecureAgent`**. The API key is obtained only in [`SecureAgent::new`](src/infrastructure/adapters/agent/secure_agent.rs) via a [`Runtime`](src/core/runtime.rs) (e.g. [`LocalFileRuntime`](src/infrastructure/adapters/runtime/local_file_runtime.rs) for local dev); the core **`Agent`** does not carry a runtime. This is the first concrete shape of the secure-agent idea; **later iterations will lean further into that concept.**

A **peer agent** is the other side of the exchange (display name **`Peer`**): a core [`Agent`](src/core/agent.rs) wired with the same shell environment and [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs) for scripted replies instead of Claude. It does not use a runtime.

| Layer | Main pieces |
| --- | --- |
| **`core`** | **`Agent`**, **`Session`**, **`Environment`**, **`Llm`**, **`Runtime`** (port only; not a field on **`Agent`**) |
| **`application`** | **`create_agent`** |
| **`infrastructure`** | **`SecureAgent`**, **`ShellEnvironment`**, **`ClaudeLlm`**, **`KnockKnockAudienceLlm`**, **`JsonHttp`**, **`DummyLlm`**, **`LocalFileRuntime`** |

On each turn, the incoming peer line is appended, system prompts are merged, **`Llm::complete`** runs, the reply is appended, and output is **`print`**ed as **`{name} -> {reply}`**. **`main`** builds a **`LocalFileRuntime`**, passes it to **`SecureAgent::new`**, and runs **`play_knock_knock`**. Logging uses hierarchical **`LoggingLevel`** (**`None`** / **`Standard`** / **`Verbose`**); see [`src/core/environment.rs`](src/core/environment.rs).

## Current behavior

- The demo is a scripted knock-knock joke between the **secure agent** (**`SecureAgent`**, Claude, joke teller) and the **peer agent** (**`Peer`**, canned lines). There is no interactive input: **`main`** sends a synthetic greeting (**`Hello.`**) to **`SecureAgent`** so it can open with an invitation; the teller’s session prompt instructs that step. **`Peer`** replies in order: **`yes`**, **`Who's there?`**, **`{word} who?`** (using the setup word parsed from the teller’s line), then **`haha`**. **`SecureAgent`** follows a teller session prompt for the invitation, **`Knock knock.`**, setup, punchline, and a brief parting after **`haha`**. The exchange ends after that parting line.
- **`start_session`** role pairs in **`main`** store Claude’s lines under transcript **`assistant`** and canned lines under **`user`**, matching what the Anthropic Messages API expects (each completion runs after a final **`user`** message).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).
- Optional: [just](https://github.com/casey/just) for shortcuts (`just lint`, `just test`, `just verify`, etc.; see [`justfile`](justfile) at the repo root).

## Build and run

```sh
cargo build
cargo run
```

No stdin is read for the knock-knock flow; the binary alternates secure agent and peer agent for a fixed sequence of turns and exits after the secure agent’s parting line (**`SecureAgent`**).

## Layout

The repo is a single Rust package (**`aria-poc-2`**) at the repository root.

## Checks

**Pre-commit** (full order is in [`.cursor/rules/workflow.mdc`](.cursor/rules/workflow.mdc)): (1) review this README for accuracy vs the repo, (2) confirm dependency direction (**core** → **application** → **infrastructure**, inward only), (3) run automated checks below.

With [just](https://github.com/casey/just) installed:

```sh
just lint
just test
```

Automated sequence (format, lint, build, test)—use **`just precommit`** or **`just verify`** (equivalent):

```sh
just precommit
```

Equivalent raw `cargo` commands:

```sh
cargo fmt
cargo clippy --workspace
cargo build
cargo test
```

## API keys

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). **`main`** constructs a **`LocalFileRuntime`** (optional custom path via [`LocalFileRuntime::new`](src/infrastructure/adapters/runtime/local_file_runtime.rs)) and passes it to **`SecureAgent::new`**. That constructor asks the runtime for the secret named **`anthropic_api_key`** (see **`ANTHROPIC_API_KEY_SECRET`**); [`LocalFileRuntime`](src/infrastructure/adapters/runtime/local_file_runtime.rs) reads and trims the file (see **`AnthropicApiKeyError`**), and **`SecureAgent`** passes the resulting string to **`ClaudeLlm::new`**. The core **`Agent`** value does not retain the runtime. If initialization fails (missing or invalid key file), **`main`** prints to stderr and exits. The **peer agent** does not load secrets. **`DummyLlm`** is a minimal always-same-reply **`Llm`** if you swap adapters in your own entrypoint.
