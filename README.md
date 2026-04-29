# Arena

This repository is a **Cargo workspace** for [Scaling Trust Arena](https://arena.nicolaos.org/) peer-to-peer agent demos. The workspace root has **no** `[package]` — only member crates.

## Architecture

The workspace uses a **hexagonal** (ports-and-adapters) shape: **`verity-core`** is the **inner hexagon**—domain types and **ports** only (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`, etc.), with no Omnia or WASI dependencies. Arena traffic is modeled as the named **`"arena"`** tool in the registry, not a separate core trait. **`verity-tools`** holds pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`) built on **`verity-core`**’s `Tool` trait. **`secure-agent`** is the **application**: it assembles a tool registry and is built as the WASM guest. **`verity-runtime`** is **infrastructure** on the host: it loads the guest `.wasm` and wires Omnia/WASI **adapters** (vault, HTTP, keyvalue, telemetry); it intentionally does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`. **`arena-stub`** is a **simulation** of a real arena peer—a scripted local HTTP process for development—rather than the production [Scaling Trust Arena](https://arena.nicolaos.org/) service. The **`arena-stub`** crate is only this local simulator; the JSON field **`arena_url`** refers to whichever peer you point at (stub or production), not the crate name.

| Directory | Crate | Role |
| --- | --- | --- |
| `core/` | `verity-core` | Shared domain types, trait ports (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`), game logic, and tests. |
| `tools/` | `verity-tools` | Pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`). Each tool is a named, auditable capability exposed through the `Tool` trait defined in `verity-core`. |
| `secure-agent/` | `secure-agent` | **WASI guest** (`wasm32-wasip2` cdylib): example agent that assembles a tool registry and plays arena games inside the sandbox. |
| `runtime/` | `verity-runtime` | **Omnia host** binary: loads the guest `.wasm`, links vault + HTTP + OpenTelemetry + **in-memory `wasi:keyvalue`** (`KeyValueDefault`; required because the guest’s HTTP stack imports keyvalue). Includes **`verity-signer`**: a standalone localhost HTTP server (Ed25519 signing for production Arena **`/play`** with **`invite`**; PKCS#8 key file at workspace root — see Production Arena section below). |
| `arena-stub/` | `arena-stub` | Local HTTP **arena** simulator: Scaling Trust Arena–shaped routes with a scripted PSI peer for development. |

**Dependency direction:** `verity-core` has no dependency on other members. `verity-tools` depends on `verity-core`. `secure-agent` depends on `verity-core` and `verity-tools`. `arena-stub` depends on `verity-core`. `verity-runtime` does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`.

### Tool model

The agent receives a [`ToolRegistry`](core/src/tool.rs) at construction containing the tools it is allowed to use. Each tool has a name, description, and `execute` method taking structured JSON input and returning structured JSON output. The registry is populated at construction and treated as fixed afterward. Synchronous game logic (`play_game` in `verity-core`) dispatches the `"arena"` tool through this registry; failures surface as [`PlayGameError::Tool`](core/src/game_loop.rs). The WASI guest also registers an arena client tool so the capability set is explicit and auditable, while async game loops continue to call the arena adapter’s **`send_async`** directly to avoid deadlocks.

The former **`Runtime`** trait in `verity-core` (a fixed menu of `get_secret`, `post_json`, `create_transport`) has been removed in favor of the tool registry. The **concept** of the secure runtime—WASI sandboxing, capability scoping, the enforcement boundary—is unchanged; the `verity-runtime` host binary and adapters are the same.

The guest uses Axum’s `IntoResponse` for HTTP handlers instead of `omnia_sdk::HttpResult`: `omnia-sdk` 0.30.0 does not currently compile on this toolchain, while the WASI/Omnia crates used for vault and HTTP do.

### How it works

1. The runtime loads the secure-agent WASM and grants vault + HTTP + keyvalue (+ otel) capabilities.
2. A `POST /play` request to the runtime’s HTTP port triggers the guest’s handler, which runs **PSI only** (`"game": "psi"`). The JSON body must include **`"arena_url"`** and **`"game"`**. With **no** **`invite`**, the guest resets the stub where applicable, creates a challenge at **`arena_url`**, and self-joins (local stub / dev). With **`invite`**, the guest uses **signed join** against production Arena: it calls the host **`verity-signer`** service for Ed25519 bytes, then **`POST .../arena/join`** and bearer **`sessionKey`** for chat. Optional **`"signer_url"`** selects the signer (default **`http://127.0.0.1:8090`**), used when **`invite`** is present. The Omnia stack listens on **`0.0.0.0:8080`** by default (override with env **`HTTP_ADDR`**, e.g. `127.0.0.1:8080`). Use **`http://127.0.0.1:8080/play`** in `curl`, not port 8000.
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

**PSI end-to-end:** The wasm guest is already built if you ran **`just build`** above; otherwise **`just run-runtime`** runs **`build-guest`** first.

```sh
# 1) Terminal A — arena stub (listens on 127.0.0.1:3000)
just run-arena
# or: cargo run -p arena-stub

# 2) Terminal B — Omnia runtime with the guest
just run-runtime
# or: cargo run -p verity-runtime --bin verity-runtime -- run target/wasm32-wasip2/debug/secure_agent.wasm

# 3) Terminal C — trigger the game (runtime HTTP defaults to port 8080; see HTTP_ADDR)
# PSI against local stub (auto-create + join)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  -d '{"game": "psi", "arena_url": "http://127.0.0.1:3000"}'
```

**Production Arena (signed join).** Requires a one-time keypair setup, then three processes.

First, generate the Ed25519 signing key (run once at the workspace root):

```sh
openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > arena_signing_key.hex
```

This produces a single-line lowercase hex string of PKCS#8 DER. Do not commit this file. Back it up if you want a stable agent identity across machines. **`verity-signer`** reads this file at startup and exits with an error if it is missing.

```sh
# Terminal A — signer (listens on 127.0.0.1:8090 by default)
just run-signer

# Terminal B — Omnia runtime with the secure-agent guest
just run-runtime

# Terminal C — trigger the game against live Arena (use a valid invite from the operator)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  --max-time 600 \
  -d '{"game":"psi","arena_url":"https://arena-engine.nicolaos.org","invite":"inv_..."}'
```

The request can take **minutes** to return: each turn calls the LLM and the arena over WASI HTTP. **`curl -s` prints nothing until the response is ready**, so it can look like nothing is happening—watch the **runtime** terminal for transcript lines (`peer <-` / `SecureAgent ->`). Omit **`-s`** if you want curl’s progress meter, or add e.g. **`--max-time 600`** so curl doesn’t give up early.

**Anthropic API key on the host:** place `anthropic_api_key.txt` at the **workspace root**, or set **`SECURE_ANTHROPIC_API_KEY_FILE`** to the key file path. The runtime vault backend (`runtime/src/plugins/vault_anthropic_local.rs`) serves secret id `anthropic_api_key` from locker `secure-anthropic`, matching the guest.

## Arena stub

The **arena-stub** binary listens on **`127.0.0.1:3000`** (see `arena-stub/src/lib.rs` for `ARENA_STUB_LISTEN_PORT` and deterministic ids `challenge_stub_001`, `inv_stub_agent`, `inv_stub_peer`). It mimics the Scaling Trust Arena HTTP surface with canned responses and a single in-memory PSI challenge. There is no auth/signing (standalone / localhost style).

| Method | Path | Role |
| --- | --- | --- |
| `POST` | `/api/v1/challenges/psi` | Create or replace the stub challenge; returns `{"id","invites"}`. |
| `POST` | `/api/v1/arena/join` | Body `{"invite":"inv_stub_agent"}` (or peer invite); returns `{"ChallengeID":"challenge_stub_001"}`. |
| `GET` | `/api/v1/arena/sync` | Query `channel`, `from` (invite), `index` — operator messages for that invite from that offset. |
| `POST` | `/api/v1/chat/send` | Body `{"channel","from","content"}` — appends chat; if `from` is the agent invite, appends the scripted peer line (`psi_peer`). |
| `GET` | `/api/v1/chat/sync` | Query `channel`, `from`, `index` — chat transcript from `index` (global indices). |
| `POST` | `/api/v1/arena/message` | Body `{"challengeId","from","messageType","content"}` — records a submission; stub appends a short operator ack for sync. |
| `POST` | `/reset` | **`204 No Content`** — clears all stub state (dev convenience). |

The secure-agent guest calls **`POST /reset`** at the start of each **`/play`** when no **`invite`** is provided, then drives **`POST .../challenges/psi`**, **`POST .../arena/join`** (stub-style invite-only body), **`POST .../chat/send`**, and **`GET .../chat/sync`** via **`StubArena`** (`secure-agent/src/stub_arena.rs`). With **`invite`** in the **`/play`** body, **`ProductionArena`** (`secure-agent/src/production_arena.rs`) skips challenge creation and **`/reset`**; it uses **`verity-signer`** for PKCS#8 Ed25519 signing, **`POST .../arena/join`** with **`publicKey`**, **`signature`**, **`timestamp`**, obtains **`ChallengeID`** and **`sessionKey`**, and bearer auth for **`chat/send`** / **`chat/sync`** (no **`from`** field). Challenge text for the real agent still comes from `PsiGame` in **`verity-core`** today; operator sync on the stub carries parallel canned instructions for manual or future client use. After both sides agree on the hash strategy, the guest and the stub each print their own private letter set to the host console once, labeled as local-only, before the hash exchange.

**Production Arena summary:** Obtain an invite code, run **`just run-signer`** (with **`arena_signing_key.hex`** at the workspace root), **`just run-runtime`**, then **`POST /play`** with **`arena_url`** and **`invite`**; optional **`signer_url`**. The guest never touches the signing key — **`verity-signer`** alone reads **`arena_signing_key.hex`**.

**Manual curl flow (illustrative):**

```sh
BASE=http://127.0.0.1:3000
curl -s -X POST "$BASE/reset"
curl -s -X POST "$BASE/api/v1/challenges/psi"
curl -s -X POST "$BASE/api/v1/arena/join" -H 'Content-Type: application/json' \
  -d '{"invite":"inv_stub_agent"}'
curl -s "$BASE/api/v1/arena/sync?channel=challenge_stub_001&from=inv_stub_agent&index=0"
curl -s -X POST "$BASE/api/v1/chat/send" -H 'Content-Type: application/json' \
  -d '{"channel":"challenge_stub_001","from":"inv_stub_agent","content":"Hello."}'
curl -s "$BASE/api/v1/chat/sync?channel=challenge_stub_001&from=inv_stub_agent&index=0"
```

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
