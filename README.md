# aria-poc-2

This repository is a **Cargo workspace** with two members:

| Member | Role |
| --- | --- |
| **`aria-poc-2`** | Agent library and **`main`** binary: knock-knock demo with **`SecureAgent`** talking to **`arena-stub`** over HTTP ([`Arena`](src/core/arena.rs) / [`ArenaHttpClient`](src/infrastructure/adapters/arena/arena_http_client.rs)). |
| **`arena-stub`** | Standalone HTTP server that simulates a future **Arena**: it plays the knock-knock **audience** role via **`POST /message`** (scripted replies; no dependency on `aria-poc-2`). |

**Peer-to-peer AI agents** in Rust. The knock-knock demo is a **peer interaction**: the **`SecureAgent`** completes [`Agent::receive_message`](src/core/agent.rs) turns while the scripted audience lines are supplied by **`arena-stub`** over HTTP ([`Arena::send`](src/core/arena.rs)).

A **secure agent** is the composition implemented as [`SecureAgent`](src/infrastructure/adapters/agent/secure_agent.rs): [`ShellEnvironment`](src/infrastructure/adapters/environment/shell_environment.rs), Anthropic [`ClaudeLlm`](src/infrastructure/adapters/llm/claude_llm.rs), and fixed display name **`SecureAgent`**. The API key is obtained only in [`SecureAgent::new`](src/infrastructure/adapters/agent/secure_agent.rs) via a [`Runtime`](src/core/runtime.rs) (see **Runtimes** below); outbound JSON POSTs use a [`PostJsonTransport`](src/core/transport.rs) injected at the composition root (e.g. [`OmniaWasiHttpPostJson`](src/infrastructure/adapters/runtime/plugins/omnia_wasi_http_post_json.rs) from **`main`**, which uses Omnia’s **`HttpDefault`** `wasi:http` host implementation from **`omnia-wasi-http`**). The core **`Agent`** does not carry a runtime. This is the first concrete shape of the secure-agent idea; **later iterations will lean further into that concept.**

The **audience** side of the exchange in **`main`** is not an in-process [`Agent`](src/core/agent.rs): it is **`arena-stub`**, reached through the [`Arena`](src/core/arena.rs) port ([`ArenaHttpClient`](src/infrastructure/adapters/arena/arena_http_client.rs)). [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs) remains in the codebase (same script as the stub) for tests and reuse but is not used by the **`main`** binary.

| Layer | Main pieces |
| --- | --- |
| **`core`** | **`Agent`**, **`Arena`**, **`Session`**, **`Environment`**, **`Llm`**, **`Runtime`**, **`PostJsonTransport`** / **`TransportError`** (ports; not fields on **`Agent`**) |
| **`application`** | **`create_agent`** |
| **`infrastructure`** | **`SecureAgent`**, **`ArenaHttpClient`**, **`ShellEnvironment`**, **`ClaudeLlm`**, **`KnockKnockAudienceLlm`**, **`OmniaWasiHttpPostJson`**, **`DummyLlm`**, **`OmniaRuntime`**, **`OmniaWasiVaultAnthropicLocal`** |
| **`arena-stub`** (workspace crate) | HTTP server only: **`POST /message`** JSON `{"message":"..."}` → `{"reply":"..."}` for the scripted audience lines. |

On each turn, the incoming peer line is appended, system prompts are merged, **`Llm::complete`** runs, and the reply is appended. Incoming lines are **`print`**ed as **`peer <- {message}`** (always), and the agent reply as **`{name} -> {reply}`**. **`main`** wires a [`Runtime`](src/core/runtime.rs) and an [`OmniaWasiHttpPostJson`](src/infrastructure/adapters/runtime/plugins/omnia_wasi_http_post_json.rs) transport into **`SecureAgent::new`** (see **Runtimes** below), a second **`OmniaWasiHttpPostJson`** into [`ArenaHttpClient`](src/infrastructure/adapters/arena/arena_http_client.rs) for **`POST /message`** to **`arena-stub`**, and runs **`play_knock_knock_via_arena`**. Logging uses hierarchical **`LoggingLevel`** (**`None`** / **`Standard`** / **`Verbose`**); see [`src/core/environment.rs`](src/core/environment.rs).

## Arena stub (`arena-stub`)

The **`arena-stub`** binary listens on **`127.0.0.1:3000`** (see **`ARENA_STUB_LISTEN_PORT`** and **`ARENA_STUB_MESSAGE_URL`** in [`arena-stub/src/lib.rs`](arena-stub/src/lib.rs)). It exposes **`POST /message`** with body **`{"message":"<teller line>"}`** and returns **`{"reply":"<audience line>"}`**, using the same scripted knock-knock audience sequence as [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs) (logic duplicated locally in `arena-stub` for a self-contained PoC). Sending the exact teller line **`Would you like to hear a knock-knock joke?`** at any point resets the internal step counter so you can run another full exchange without restarting the process.

**Try it:** in one terminal run **`just run-arena`** (or **`cargo run -p arena-stub`**). In another, run the teller lines in order (each request advances the stub’s internal step; **send a new request per line**). URLs match the constants in **`arena-stub/src/lib.rs`**. For each request the server prints two lines to stdout—**`peer <- <teller line>`** then **`agent -> <audience line>`**—in the same style as **`aria-poc-2`** **`Agent`** standard logging and **`print`** output.

```sh
# 1) Teller: invitation → audience: yes
curl -s -X POST "http://127.0.0.1:3000/message" -H "Content-Type: application/json" \
  -d '{"message": "Would you like to hear a knock-knock joke?"}'
# → {"reply":"yes"}

# 2) Teller: “Knock knock.” → audience: Who's there?
curl -s -X POST "http://127.0.0.1:3000/message" -H "Content-Type: application/json" \
  -d '{"message": "Knock knock."}'
# → {"reply":"Who's there?"}

# 3) Teller: setup word → audience: {word} who?
curl -s -X POST "http://127.0.0.1:3000/message" -H "Content-Type: application/json" \
  -d '{"message": "Boo"}'
# → {"reply":"Boo who?"}

# 4) Teller: punchline → audience: haha
curl -s -X POST "http://127.0.0.1:3000/message" -H "Content-Type: application/json" \
  -d '{"message": "Don'\''t cry, it'\''s just a joke!"}'
# → {"reply":"haha"}

# 5) Further messages return an empty reply until you restart (see below).
curl -s -X POST "http://127.0.0.1:3000/message" -H "Content-Type: application/json" \
  -d '{"message": "Thanks!"}'
# → {"reply":""}
```

To **play again** without restarting the server, send the invitation line again; that resets the step counter and the next response is **`{"reply":"yes"}`** (same as step 1).

**Tests:** The scripted audience behavior and invitation reset are covered by unit tests on the shared library (`arena-stub/src/lib.rs`). Run **`just test-arena`** or **`cargo test -p arena-stub`**. The full workspace test suite includes both packages: **`just test`** or **`cargo test --workspace`**.

The main **`aria-poc-2`** demo uses this HTTP path: start **`arena-stub`** first, then **`cargo run -p aria-poc-2`** or **`just run-agent`**. Optional: **`just demo`** starts the stub in the background, waits, then runs the agent. Future work: add challenges (e.g. Yao’s Millionaire, PSI); extract shared domain types into a workspace **`core`** crate when needed.

## Runtimes

**`SecureAgent`** only needs the core **`Runtime`** port: it calls **`get_secret`** with [`ANTHROPIC_API_KEY_SECRET`](src/core/runtime.rs) once during construction and does not retain the runtime.

**`main`** constructs **`OmniaRuntime`** with **`OmniaWasiVaultAnthropicLocal`**, which resolves secrets through Omnia’s host-side **`wasi:vault`** traits (**`WasiVaultCtx`** / **`Locker`**) and reads the same default key file read-only. You can swap in other **`WasiVaultCtx`** implementations (in-memory, cloud vaults, or a full WASI guest boundary) without changing **`SecureAgent`**.

## Current behavior

- **Two processes:** start **`arena-stub`** (**`just run-arena`**) so it listens on **`127.0.0.1:3000`** and plays the knock-knock **audience** (same script as [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs)). Run the agent (**`just run-agent`**): **`SecureAgent`** uses [`ArenaHttpClient`](src/infrastructure/adapters/arena/arena_http_client.rs) to **`POST /message`** after each teller line; peer replies arrive as **`{"reply":"..."}`** over HTTP and are fed into **`SecureAgent::receive_message`**. There is no interactive input: **`main`** sends a synthetic greeting (**`Hello.`**) so the teller can open with an invitation. The stub replies in order: **`yes`**, **`Who's there?`**, **`{word} who?`**, then **`haha`**, then empty. **`SecureAgent`** follows the teller session prompt through invitation, **`Knock knock.`**, setup, punchline, and a brief parting after **`haha`**. The loop stops when the peer reply is empty or after a bounded number of turns.
- **`start_session`** in **`main`** stores Claude’s lines under transcript **`assistant`** and peer lines under **`user`**, matching what the Anthropic Messages API expects (each completion runs after a final **`user`** message).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).
- Optional: [just](https://github.com/casey/just) for shortcuts (`just lint`, `just test`, `just verify`, etc.; see [`justfile`](justfile) at the repo root).

## Build and run

**Knock-knock demo (agent + arena-stub):** in one terminal start the stub, in another run the agent.

```sh
# Terminal A
just run-arena
# or: cargo run -p arena-stub
```

```sh
# Terminal B
cargo build -p aria-poc-2
cargo run -p aria-poc-2
```

(or **`just run-agent`**)

**Convenience (stub in background, then agent):** **`just demo`**

**Arena stub only:**

```sh
cargo run -p arena-stub
```

(or **`just run-arena`**)

**Entire workspace:**

```sh
cargo build --workspace
```

No stdin is read for the knock-knock flow; the **`aria-poc-2`** binary exchanges messages with **`arena-stub`** over HTTP until the scripted sequence completes (empty peer reply or turn limit).

## Layout

The workspace root lists **`aria-poc-2`** (package at `.`) and **`arena-stub`**. The agent library and re-exports live in **`src/lib.rs`**; the **`main`** binary entry point is **`src/main.rs`**. Shared unit-test doubles and fixtures live in **`src/test_support.rs`** (loaded only when testing via **`#[cfg(test)] mod test_support`** in **`lib.rs`**).

## Checks

**Pre-commit** (full order is in [`.cursor/rules/workflow.mdc`](.cursor/rules/workflow.mdc)): (1) review this README for accuracy vs the repo, (2) confirm dependency direction (**core** → **application** → **infrastructure**, inward only), (3) run automated checks below.

With [just](https://github.com/casey/just) installed:

```sh
just lint
just test
```

Arena stub tests only:

```sh
just test-arena
```

Automated sequence (format, lint, build, test)—use **`just precommit`** or **`just verify`** (equivalent):

```sh
just precommit
```

Equivalent raw `cargo` commands:

```sh
cargo fmt --all
cargo clippy --workspace
cargo build --workspace
cargo test --workspace
```

## API keys

If you use a local API key file, name it `anthropic_api_key.txt` at the repo root (ignored by git per `.gitignore`). **`SecureAgent::new`** asks the runtime for the secret named **`anthropic_api_key`** ([**`ANTHROPIC_API_KEY_SECRET`**](src/core/runtime.rs)). The Omnia vault uses the same id as **`ANTHROPIC_VAULT_SECRET_ID`** with locker **`aria-anthropic`** (**`ANTHROPIC_VAULT_LOCKER_ID`**). Configure the key file via [`OmniaWasiVaultAnthropicLocal::new`](src/infrastructure/adapters/runtime/plugins/omnia_wasi_vault_anthropic_local.rs) (default: repo-root file). Empty or missing content maps to “not found” for the vault **`get`** path; **`main`** exits if **`SecureAgent::new`** fails.

The core **`Agent`** value does not retain the runtime. The **arena-stub** process does not load secrets; neither does an in-process **Peer** wired with **`KnockKnockAudienceLlm`**. **`DummyLlm`** is a minimal always-same-reply **`Llm`** if you swap adapters in your own entrypoint.
