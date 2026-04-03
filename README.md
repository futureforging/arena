# aria-poc-2

This repository is a **Cargo workspace** with two members:

| Member | Role |
| --- | --- |
| **`aria-poc-2`** | Agent library and **`main`** binary: knock-knock demo with an in-process peer (`KnockKnockAudienceLlm`). |
| **`arena-stub`** | Standalone HTTP server that simulates a future **Arena**: it plays the knock-knock **audience** role via **`POST /message`** (scripted replies; no dependency on `aria-poc-2`). |

**Peer-to-peer AI agents** in Rust. The knock-knock demo is a **peer interaction** between a **secure agent** and a **peer agent**—two participants that alternate **`receive_message`** on the core [`Agent`](src/core/agent.rs) API.

A **secure agent** is the composition implemented as [`SecureAgent`](src/infrastructure/adapters/agent/secure_agent.rs): [`ShellEnvironment`](src/infrastructure/adapters/environment/shell_environment.rs), Anthropic [`ClaudeLlm`](src/infrastructure/adapters/llm/claude_llm.rs), and fixed display name **`SecureAgent`**. The API key is obtained only in [`SecureAgent::new`](src/infrastructure/adapters/agent/secure_agent.rs) via a [`Runtime`](src/core/runtime.rs) (see **Runtimes** below); outbound JSON POSTs use a [`PostJsonTransport`](src/core/transport.rs) injected at the composition root (e.g. [`OmniaWasiHttpPostJson`](src/infrastructure/adapters/runtime/plugins/omnia_wasi_http_post_json.rs) from **`main`**, which uses Omnia’s **`HttpDefault`** `wasi:http` host implementation from **`omnia-wasi-http`**). The core **`Agent`** does not carry a runtime. This is the first concrete shape of the secure-agent idea; **later iterations will lean further into that concept.**

A **peer agent** is the other side of the exchange (display name **`Peer`**): a core [`Agent`](src/core/agent.rs) wired with the same shell environment and [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs) for scripted replies instead of Claude. It does not use a runtime.

| Layer | Main pieces |
| --- | --- |
| **`core`** | **`Agent`**, **`Session`**, **`Environment`**, **`Llm`**, **`Runtime`**, **`PostJsonTransport`** / **`TransportError`** (ports; not fields on **`Agent`**) |
| **`application`** | **`create_agent`** |
| **`infrastructure`** | **`SecureAgent`**, **`ShellEnvironment`**, **`ClaudeLlm`**, **`KnockKnockAudienceLlm`**, **`OmniaWasiHttpPostJson`**, **`DummyLlm`**, **`OmniaRuntime`**, **`OmniaWasiVaultAnthropicLocal`** |
| **`arena-stub`** (workspace crate) | HTTP server only: **`POST /message`** JSON `{"message":"..."}` → `{"reply":"..."}` for the scripted audience lines. |

On each turn, the incoming peer line is appended, system prompts are merged, **`Llm::complete`** runs, the reply is appended, and output is **`print`**ed as **`{name} -> {reply}`**. **`main`** wires a [`Runtime`](src/core/runtime.rs) and an [`OmniaWasiHttpPostJson`](src/infrastructure/adapters/runtime/plugins/omnia_wasi_http_post_json.rs) transport into **`SecureAgent::new`** (see **Runtimes** below) and runs **`play_knock_knock`**. Logging uses hierarchical **`LoggingLevel`** (**`None`** / **`Standard`** / **`Verbose`**); see [`src/core/environment.rs`](src/core/environment.rs).

## Arena stub (`arena-stub`)

The **`arena-stub`** binary listens on **`127.0.0.1`** (default port **`3000`**; override with **`ARENA_STUB_PORT`**). It exposes **`POST /message`** with body **`{"message":"<teller line>"}`** and returns **`{"reply":"<audience line>"}`**, using the same scripted knock-knock audience sequence as [`KnockKnockAudienceLlm`](src/infrastructure/adapters/llm/knock_knock_audience_llm.rs) (logic duplicated locally in `arena-stub` for a self-contained PoC). Sending the exact teller line **`Would you like to hear a knock-knock joke?`** at any point resets the internal step counter so you can run another full exchange without restarting the process.

**Try it:** in one terminal run **`just run-arena`** (or **`cargo run -p arena-stub`**). In another, run the teller lines in order (each request advances the stub’s internal step; **send a new request per line**):

```sh
BASE=http://127.0.0.1:3000/message

# 1) Teller: invitation → audience: yes
curl -s -X POST "$BASE" -H "Content-Type: application/json" \
  -d '{"message": "Would you like to hear a knock-knock joke?"}'
# → {"reply":"yes"}

# 2) Teller: “Knock knock.” → audience: Who's there?
curl -s -X POST "$BASE" -H "Content-Type: application/json" \
  -d '{"message": "Knock knock."}'
# → {"reply":"Who's there?"}

# 3) Teller: setup word → audience: {word} who?
curl -s -X POST "$BASE" -H "Content-Type: application/json" \
  -d '{"message": "Boo"}'
# → {"reply":"Boo who?"}

# 4) Teller: punchline → audience: haha
curl -s -X POST "$BASE" -H "Content-Type: application/json" \
  -d '{"message": "Don'\''t cry, it'\''s just a joke!"}'
# → {"reply":"haha"}

# 5) Further messages return an empty reply until you restart (see below).
curl -s -X POST "$BASE" -H "Content-Type: application/json" \
  -d '{"message": "Thanks!"}'
# → {"reply":""}
```

To **play again** without restarting the server, send the invitation line again; that resets the step counter and the next response is **`{"reply":"yes"}`** (same as step 1).

**Tests:** The scripted audience behavior and invitation reset are covered by unit tests on the shared library (`arena-stub/src/lib.rs`). Run **`just test-arena`** or **`cargo test -p arena-stub`**. The full workspace test suite includes both packages: **`just test`** or **`cargo test --workspace`**.

The main **`aria-poc-2`** demo is unchanged: **`cargo run -p aria-poc-2`** or **`just run-agent`** still runs the in-process peer. Future work: have **`SecureAgent`** call this HTTP endpoint instead of the in-process peer; add challenges (e.g. Yao’s Millionaire, PSI); extract shared domain types into a workspace **`core`** crate when needed.

## Runtimes

**`SecureAgent`** only needs the core **`Runtime`** port: it calls **`get_secret`** with [`ANTHROPIC_API_KEY_SECRET`](src/core/runtime.rs) once during construction and does not retain the runtime.

**`main`** constructs **`OmniaRuntime`** with **`OmniaWasiVaultAnthropicLocal`**, which resolves secrets through Omnia’s host-side **`wasi:vault`** traits (**`WasiVaultCtx`** / **`Locker`**) and reads the same default key file read-only. You can swap in other **`WasiVaultCtx`** implementations (in-memory, cloud vaults, or a full WASI guest boundary) without changing **`SecureAgent`**.

## Current behavior

- The demo is a scripted knock-knock joke between the **secure agent** (**`SecureAgent`**, Claude, joke teller) and the **peer agent** (**`Peer`**, canned lines). There is no interactive input: **`main`** sends a synthetic greeting (**`Hello.`**) to **`SecureAgent`** so it can open with an invitation; the teller’s session prompt instructs that step. **`Peer`** replies in order: **`yes`**, **`Who's there?`**, **`{word} who?`** (using the setup word parsed from the teller’s line), then **`haha`**. **`SecureAgent`** follows a teller session prompt for the invitation, **`Knock knock.`**, setup, punchline, and a brief parting after **`haha`**. The exchange ends after that parting line.
- **`start_session`** role pairs in **`main`** store Claude’s lines under transcript **`assistant`** and canned lines under **`user`**, matching what the Anthropic Messages API expects (each completion runs after a final **`user`** message).

## Requirements

- Rust **nightly** (see `rust-toolchain.toml`).
- Optional: [just](https://github.com/casey/just) for shortcuts (`just lint`, `just test`, `just verify`, etc.; see [`justfile`](justfile) at the repo root).

## Build and run

**Knock-knock demo (in-process peer):**

```sh
cargo build -p aria-poc-2
cargo run -p aria-poc-2
```

(or **`just run-agent`**)

**Arena stub only:**

```sh
cargo run -p arena-stub
```

(or **`just run-arena`**)

**Entire workspace:**

```sh
cargo build --workspace
```

No stdin is read for the knock-knock flow; the **`aria-poc-2`** binary alternates secure agent and peer agent for a fixed sequence of turns and exits after the secure agent’s parting line (**`SecureAgent`**).

## Layout

The workspace root lists **`aria-poc-2`** (package at `.`) and **`arena-stub`**. The agent code lives under **`src/`** in **`aria-poc-2`**. Shared unit-test doubles and fixtures live in **`src/test_support.rs`** (loaded only when testing via **`#[cfg(test)] mod test_support`** in **`main.rs`**).

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

The core **`Agent`** value does not retain the runtime. The **peer agent** does not load secrets. **`DummyLlm`** is a minimal always-same-reply **`Llm`** if you swap adapters in your own entrypoint.
