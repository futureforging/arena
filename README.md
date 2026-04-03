# aria-poc-2

This repository is a **Cargo workspace** for peer-to-peer agent demos. The workspace root has **no** `[package]` — only member crates.

## Architecture

| Directory | Crate | Role |
| --- | --- | --- |
| `core/` | `aria-core` | Shared domain types, trait ports (`Arena`, `Llm`, `Environment`, `Game`), `play_game`, `KnockKnockGame`, and tests. |
| `secure-agent/` | `aria-secure-agent` | **WASI guest** (`wasm32-wasip2` cdylib): constrained agent that handles `POST /play`, uses WASI vault for the Anthropic API key and WASI HTTP for the arena and Anthropic APIs. |
| `runtime/` | `aria-runtime` | **Omnia host** binary: loads the guest `.wasm`, links vault + HTTP + OpenTelemetry + **in-memory `wasi:keyvalue`** (`KeyValueDefault`; required because the guest’s HTTP stack imports keyvalue). |
| `arena-stub/` | `arena-stub` | Local HTTP **arena** peer: scripted knock-knock audience via `POST /message`. |

**Dependency direction:** `aria-core` has no dependency on other members. `aria-secure-agent` and `arena-stub` depend on `aria-core`. `aria-runtime` does **not** depend on `aria-core` or `aria-secure-agent` (it loads the guest wasm from disk).

The guest uses Axum’s `IntoResponse` for HTTP handlers instead of `omnia_sdk::HttpResult`: `omnia-sdk` 0.30.0 does not currently compile on this toolchain, while the WASI/Omnia crates used for vault and HTTP do.

### How it works

1. The runtime loads the secure-agent WASM and grants vault + HTTP + keyvalue (+ otel) capabilities.
2. A `POST /play` request to the runtime’s HTTP port triggers the guest’s handler, which runs `play_game` from `aria-core` inside the sandbox. The Omnia stack listens on **`0.0.0.0:8080`** by default (override with env **`HTTP_ADDR`**, e.g. `127.0.0.1:8080`). Use **`http://127.0.0.1:8080/play`** in `curl`, not port 8000.
3. The agent reads the API key from WASI vault, talks to the arena and to Anthropic only through WASI HTTP.
4. The host controls both capabilities; the guest has no direct filesystem access for secrets, no raw env-based secret injection in the guest, and no unsandboxed network.

## Requirements

- Rust **nightly** and the **`wasm32-wasip2`** target (see `rust-toolchain.toml`).
- Optional: [just](https://github.com/casey/just) for shortcuts (`justfile` at the repo root).

## Build and run

**Build everything (native runtime + wasm guest):**

```sh
just build
```

**Knock-knock end-to-end:**

```sh
# 1) Build the guest
cargo build -p aria-secure-agent --target wasm32-wasip2

# 2) Terminal A — arena stub (listens on 127.0.0.1:3000)
just run-arena
# or: cargo run -p arena-stub

# 3) Terminal B — Omnia runtime with the guest
just run-runtime
# or: cargo run -p aria-runtime -- run target/wasm32-wasip2/debug/aria_secure_agent.wasm

# 4) Terminal C — trigger the game (runtime HTTP defaults to port 8080; see HTTP_ADDR)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  -d '{"game": "knock-knock", "arena_url": "http://127.0.0.1:3000"}'
```

The request can take **minutes** to return: each turn calls the LLM and the arena over WASI HTTP. To avoid `curl` giving up, add e.g. **`--max-time 600`**.

**Anthropic API key on the host:** place `anthropic_api_key.txt` at the **workspace root**, or set **`ARIA_ANTHROPIC_API_KEY_FILE`** to the key file path. The runtime vault backend (`runtime/src/plugins/vault_anthropic_local.rs`) serves secret id `anthropic_api_key` from locker `aria-anthropic`, matching the guest.

## Arena stub

The **arena-stub** binary listens on **`127.0.0.1:3000`** (see `arena-stub/src/lib.rs` for `ARENA_STUB_LISTEN_PORT` and related constants). It exposes **`POST /message`** with body `{"message":"..."}` and returns `{"reply":"..."}` for the scripted knock-knock audience. See that file for scripted steps and the invitation reset behavior.

**Outbound HTTP from the guest:** Prefer **`http://127.0.0.1:3000`** in `arena_url`. The stub listens on **IPv4 only**; using `localhost` can resolve to **`::1`**, so the request never hits port 3000. The guest normalizes `localhost` to `127.0.0.1` before calling the arena. If you use a system **`HTTP_PROXY`**, set **`NO_PROXY`** so `127.0.0.1` and `localhost` are reached directly (the `just run-runtime` recipe sets this).

## Checks

With [just](https://github.com/casey/just):

```sh
just lint
just test
```

Full automated sequence (format, lint, build host + guest, test):

```sh
just verify
# same as: just precommit
```

Equivalent raw commands:

```sh
cargo fmt --all
cargo clippy --workspace
just build
cargo test --workspace
```

**Pre-commit** (full order is in [`.cursor/rules/workflow.mdc`](.cursor/rules/workflow.mdc)): (1) review this README, (2) confirm dependency direction, (3) run the automated checks above.

`aria-core` only:

```sh
just test-core
```

`arena-stub` only:

```sh
just test-arena
```
