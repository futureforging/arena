# Arena

This repository is a **Cargo workspace** for [Scaling Trust Arena](https://arena.nicolaos.org/) peer-to-peer agent demos. The workspace root has **no** `[package]` — only member crates.

## Architecture

The workspace uses a **hexagonal** (ports-and-adapters) shape: **`verity-core`** is the **inner hexagon**—domain types and **ports** only (`Arena`, `Llm`, `Environment`, `Game`, `Tool`, `ToolRegistry`, etc.), with no Omnia or WASI dependencies. **`verity-tools`** holds pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`) built on **`verity-core`**’s `Tool` trait. **`secure-agent`** is the **application**: it assembles a tool registry and is built as the WASM guest. **`verity-runtime`** is **infrastructure** on the host: it loads the guest `.wasm` and wires Omnia/WASI **adapters** (vault, HTTP, keyvalue, telemetry); it intentionally does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`. **`arena-stub`** is a **simulation** of a real arena peer—a scripted local HTTP process for development—rather than the production [Scaling Trust Arena](https://arena.nicolaos.org/) service. The **`arena-stub`** crate is only this local simulator; the **`Arena`** port and the JSON field **`arena_url`** refer to whichever peer you point at (stub or production), not the crate name.

| Directory | Crate | Role |
| --- | --- | --- |
| `core/` | `verity-core` | Shared domain types, trait ports (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`), game logic, and tests. |
| `tools/` | `verity-tools` | Pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`). Each tool is a named, auditable capability exposed through the `Tool` trait defined in `verity-core`. |
| `secure-agent/` | `secure-agent` | **WASI guest** (`wasm32-wasip2` cdylib): example agent that assembles a tool registry and plays arena games inside the sandbox. |
| `runtime/` | `verity-runtime` | **Omnia host** binary: loads the guest `.wasm`, links vault + HTTP + OpenTelemetry + **in-memory `wasi:keyvalue`** (`KeyValueDefault`; required because the guest’s HTTP stack imports keyvalue). |
| `arena-stub/` | `arena-stub` | Local HTTP **arena** peer: scripted knock-knock audience or PSI peer for development. |

**Dependency direction:** `verity-core` has no dependency on other members. `verity-tools` depends on `verity-core`. `secure-agent` depends on `verity-core` and `verity-tools`. `arena-stub` depends on `verity-core`. `verity-runtime` does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`.

### Tool model

The agent receives a [`ToolRegistry`](core/src/tool.rs) at construction containing the tools it is allowed to use. Each tool has a name, description, and `execute` method taking structured JSON input and returning structured JSON output. The registry is populated at construction and treated as fixed afterward. Synchronous game logic (`play_game` in `verity-core`) dispatches the `"arena"` tool through this registry; the WASI guest also registers an arena client tool so the capability set is explicit and auditable, while async game loops continue to call `WasiArena::send_async` directly to avoid deadlocks.

The former **`Runtime`** trait in `verity-core` (a fixed menu of `get_secret`, `post_json`, `create_transport`) has been removed in favor of the tool registry. The **concept** of the secure runtime—WASI sandboxing, capability scoping, the enforcement boundary—is unchanged; the `verity-runtime` host binary and adapters are the same.

The guest uses Axum’s `IntoResponse` for HTTP handlers instead of `omnia_sdk::HttpResult`: `omnia-sdk` 0.30.0 does not currently compile on this toolchain, while the WASI/Omnia crates used for vault and HTTP do.

### How it works

1. The runtime loads the secure-agent WASM and grants vault + HTTP + keyvalue (+ otel) capabilities.
2. A `POST /play` request to the runtime’s HTTP port triggers the guest’s handler, which runs the selected game inside the sandbox. The JSON body must include **`"arena_url"`** and **`"game"`** — either **`"knock-knock"`** or **`"psi"`** (required; there is no default). The Omnia stack listens on **`0.0.0.0:8080`** by default (override with env **`HTTP_ADDR`**, e.g. `127.0.0.1:8080`). Use **`http://127.0.0.1:8080/play`** in `curl`, not port 8000.
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

**Knock-knock or PSI end-to-end:** The wasm guest is already built if you ran **`just build`** above; otherwise **`just run-runtime`** runs **`build-guest`** first.

```sh
# 1) Terminal A — arena stub (listens on 127.0.0.1:3000)
just run-arena
# or: cargo run -p arena-stub

# 2) Terminal B — Omnia runtime with the guest
just run-runtime
# or: cargo run -p verity-runtime -- run target/wasm32-wasip2/debug/secure_agent.wasm

# 3) Terminal C — trigger the game (runtime HTTP defaults to port 8080; see HTTP_ADDR)
# Knock-knock
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  -d '{"game": "knock-knock", "arena_url": "http://127.0.0.1:3000"}'

# PSI (SHA-256 hash intersection script)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  -d '{"game": "psi", "arena_url": "http://127.0.0.1:3000"}'
```

The request can take **minutes** to return: each turn calls the LLM and the arena over WASI HTTP. **`curl -s` prints nothing until the response is ready**, so it can look like nothing is happening—watch the **runtime** terminal for transcript lines (`peer <-` / `SecureAgent ->`). Omit **`-s`** if you want curl’s progress meter, or add e.g. **`--max-time 600`** so curl doesn’t give up early.

**Anthropic API key on the host:** place `anthropic_api_key.txt` at the **workspace root**, or set **`SECURE_ANTHROPIC_API_KEY_FILE`** to the key file path. The runtime vault backend (`runtime/src/plugins/vault_anthropic_local.rs`) serves secret id `anthropic_api_key` from locker `secure-anthropic`, matching the guest.

## Arena stub

The **arena-stub** binary listens on **`127.0.0.1:3000`** (see `arena-stub/src/lib.rs` for `ARENA_STUB_LISTEN_PORT` and related constants). It exposes **`POST /message`** with body `{"message":"..."}` and returns `{"reply":"..."}`, and **`POST /reset`** with an empty body, which returns **`204 No Content`** and clears scripted peer state. The secure-agent guest calls **`/reset`** at the start of each **`/play`** so you can run knock-knock or PSI back-to-back without restarting the stub or runtime. You can also call reset manually (e.g. `curl -X POST http://127.0.0.1:3000/reset`). The stub infers **knock-knock** vs **PSI** from the agent’s first message after a reset (`detect_game`); see `process_turn`, `audience_reply`, and `psi_peer` for scripted steps. For **PSI**, after both sides agree on the hash strategy, the guest and the stub each print their own private letter set to the host console once, labeled as local-only (not sent to the peer), before the hash exchange.

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

`verity-core` only:

```sh
just test-core
```

`arena-stub` only:

```sh
just test-arena
```
