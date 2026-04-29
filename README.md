# Arena

This repository is a **Cargo workspace** for [Scaling Trust Arena](https://arena.nicolaos.org/) peer-to-peer agent demos. The workspace root has **no** `[package]` — only member crates.

## Architecture

The workspace uses a **hexagonal** (ports-and-adapters) shape: **`verity-core`** is the **inner hexagon**—domain types and **ports** only (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`, etc.), with no Omnia or WASI dependencies. Arena traffic is modeled as the named **`"arena"`** tool in the registry, not a separate core trait. **`verity-tools`** holds pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`) built on **`verity-core`**’s `Tool` trait. **`secure-agent`** is the **application**: it assembles a tool registry and is built as the WASM guest. **`verity-runtime`** is **infrastructure** on the host: it loads the guest `.wasm` and wires Omnia/WASI **adapters** (vault, HTTP, keyvalue, telemetry); it intentionally does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`. **`arena-stub`** is a **simulation** of a real arena peer—a scripted local HTTP process for development—rather than the production [Scaling Trust Arena](https://arena.nicolaos.org/) service. The **`arena-stub`** crate is only this local simulator; the JSON field **`arena_url`** refers to whichever peer you point at (stub or production), not the crate name.

| Directory | Crate | Role |
| --- | --- | --- |
| `core/` | `verity-core` | Shared domain types, trait ports (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`), game logic, and tests. |
| `tools/` | `verity-tools` | Pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`). Each tool is a named, auditable capability exposed through the `Tool` trait defined in `verity-core`. |
| `secure-agent/` | `secure-agent` | **WASI guest** (`wasm32-wasip2` cdylib): example agent that assembles a tool registry and plays arena games inside the sandbox. |
| `runtime/` | `verity-runtime` | **Omnia host** binary: loads the guest `.wasm`, links vault + HTTP + OpenTelemetry + **in-memory `wasi:keyvalue`** (`KeyValueDefault`; required because the guest’s HTTP stack imports keyvalue). Includes **`verity-signer`**: standalone localhost HTTP servers (Ed25519 signing for production Arena **`/play`** with **`invite`**; PKCS#8 key files at workspace root — see Production Arena section below). |
| `arena-stub/` | `arena-stub` | Local HTTP **arena** simulator: Scaling Trust Arena–shaped routes with a scripted PSI peer for development. |

**Dependency direction:** `verity-core` has no dependency on other members. `verity-tools` depends on `verity-core`. `secure-agent` depends on `verity-core` and `verity-tools`. `arena-stub` depends on `verity-core`. `verity-runtime` does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`.

### Tool model

The agent receives a [`ToolRegistry`](core/src/tool.rs) at construction containing the tools it is allowed to use. Each tool has a name, description, and `execute` method taking structured JSON input and returning structured JSON output. The registry is populated at construction and treated as fixed afterward. Synchronous game logic (`play_game` in `verity-core`) dispatches the `"arena"` tool through this registry; failures surface as [`PlayGameError::Tool`](core/src/game_loop.rs). The WASI guest also registers an arena client tool so the capability set is explicit and auditable, while async game loops continue to call the arena adapter’s **`send_async`** directly to avoid deadlocks.

The former **`Runtime`** trait in `verity-core` (a fixed menu of `get_secret`, `post_json`, `create_transport`) has been removed in favor of the tool registry. The **concept** of the secure runtime—WASI sandboxing, capability scoping, the enforcement boundary—is unchanged; the `verity-runtime` host binary and adapters are the same.

The guest uses Axum’s `IntoResponse` for HTTP handlers instead of `omnia_sdk::HttpResult`: `omnia-sdk` 0.30.0 does not currently compile on this toolchain, while the WASI/Omnia crates used for vault and HTTP do.

### How it works

1. The runtime loads the secure-agent WASM and grants vault + HTTP + keyvalue (+ otel) capabilities.
2. A `POST /play` request to the runtime’s HTTP port triggers the guest’s handler, which runs **PSI only** (`"game": "psi"`). The JSON body must include **`"arena_url"`** and **`"game"`**. With **no** **`invite`**, the guest resets the stub where applicable, creates a challenge at **`arena_url`**, and self-joins (local stub / dev). With **`invite`**, the guest uses **signed join** against production Arena: it calls the host **`verity-signer`** service for Ed25519 bytes, then **`POST .../arena/join`** and bearer **`sessionKey`** for chat. Optional **`"signer_url"`** selects the signer (default **`http://127.0.0.1:8090`**). Optional **`"role"`** and **`username`** apply only when **`invite`** is present (see Production Arena). The Omnia stack listens on **`0.0.0.0:8080`** by default (override with env **`HTTP_ADDR`**, e.g. `127.0.0.1:8080`). Use **`http://127.0.0.1:8080/play`** in `curl`, not port 8000.
3. The agent reads the API key from WASI vault, talks to the arena and to Anthropic only through WASI HTTP.
4. The host controls both capabilities; the guest has no direct filesystem access for secrets, no raw env-based secret injection in the guest, and no unsandboxed network.

## Requirements

- Rust **nightly** and the **`wasm32-wasip2`** target (see `rust-toolchain.toml`).
- Optional: [just](https://github.com/casey/just) for shortcuts (`justfile` at the repo root).

## Build

```sh
just build
```

**Anthropic API key on the host:** place `anthropic_api_key.txt` at the **workspace root**, or set **`SECURE_ANTHROPIC_API_KEY_FILE`** to the key file path. The runtime vault backend (`runtime/src/plugins/vault_anthropic_local.rs`) serves secret id `anthropic_api_key` from locker `secure-anthropic`, matching the guest.

## Production Arena

The primary run mode. Two flows are supported:

- **Single agent vs external peer** — your agent plays against another player (a teammate, another team's agent, or whatever the operator provides). Run one signer + one runtime + one `/play` call.
- **Self-play** — two of your own agents play each other (`missionary` is the first mover, `friend` is the second mover). Run two signers + one runtime + two parallel `/play` calls.

### One-time setup: signing keys

Each agent identity is an Ed25519 keypair. User IDs on the Arena are derived from public keys, so two distinct keys = two distinct users. Generate both at the workspace root:

```sh
openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > arena_signing_key_1.hex
openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > arena_signing_key_2.hex
```

Each file is a single-line lowercase hex string of PKCS#8 DER. Do not commit them. Back them up if you want stable agent identities across machines. The signer reads its assigned key file at startup and exits with a clear error if it is missing, empty, or malformed; it does not generate keys.

### Get an invite pair

Create a fresh PSI challenge — the response includes two invite codes, one per player:

```sh
curl -sS -X POST https://arena-engine.nicolaos.org/api/v1/challenges/psi
# {
#   "id": "challenge_...",
#   "invites": ["inv_AAA...", "inv_BBB..."]
# }
```

Save both invites — the first goes to your first agent, the second to your second agent (or to whoever's playing the peer side).

### Single agent run

Three terminals:

```sh
# Terminal A — signer for your agent (port 8090, key 1)
just run-signer-1

# Terminal B — runtime
just run-runtime

# Terminal C — trigger the play (replace inv_AAA with the real invite)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  --max-time 600 \
  -d '{"game":"psi","arena_url":"https://arena-engine.nicolaos.org","invite":"inv_AAA"}'
```

The peer side (using `inv_BBB`) is whoever you arranged it with. Without a peer, your agent will poll `chat/sync` and eventually fail with `no peer message after 600 chat/sync polls`.

The request can take **minutes** to return: each turn calls the LLM and the arena over WASI HTTP. **`curl -s` prints nothing until the response is ready**, so it can look like nothing is happening—watch the **runtime** terminal for transcript lines (`peer <-` / `SecureAgent ->`). Omit **`-s`** if you want curl’s progress meter. **`verity-signer`** alone reads the key file; the guest never touches the signing key.

### Self-play (`missionary` vs `friend`)

Five terminals:

```sh
# Terminal A — signer for missionary (port 8090, key 1)
just run-signer-1

# Terminal B — signer for friend (port 8091, key 2)
just run-signer-2

# Terminal C — runtime
just run-runtime

# Terminal D — start missionary (first mover)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  --max-time 600 \
  -d '{
    "game": "psi",
    "arena_url": "https://arena-engine.nicolaos.org",
    "invite": "inv_AAA",
    "signer_url": "http://127.0.0.1:8090",
    "role": "first",
    "username": "missionary"
  }'

# Terminal E — start friend (second mover)
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  --max-time 600 \
  -d '{
    "game": "psi",
    "arena_url": "https://arena-engine.nicolaos.org",
    "invite": "inv_BBB",
    "signer_url": "http://127.0.0.1:8091",
    "role": "second",
    "username": "friend"
  }'
```

Start D and E in close succession (within seconds of each other). The runtime serves both `/play` calls concurrently. Watch terminal C for interleaved transcript lines from both agents. The first agent to start gets the first poll — there is no harm in starting either side first, but if friend starts well before missionary, friend will poll for a while before missionary's `"Hello."` appears.

Both `/play` responses return `{"turns": N, "status": "complete", "game": "psi"}` when their side finishes.

### `/play` body fields

| Field        | Required             | Purpose                                                                |
|--------------|----------------------|------------------------------------------------------------------------|
| `game`       | yes                  | Must be `"psi"`.                                                       |
| `arena_url`  | yes                  | Arena base URL.                                                        |
| `invite`     | no                   | Present ⇒ production. Absent ⇒ local stub.                             |
| `signer_url` | no                   | Default `http://127.0.0.1:8090`. Set this for `friend` (port 8091).    |
| `role`       | no (production only) | `"first"` or `"second"`. Default `"second"`. Required for self-play.   |
| `username`   | no (production only) | Display name registered with the Arena. Default `"missionary"`.        |

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

## Local development with `arena-stub`

The `arena-stub` binary is a scripted local simulator of the Arena's HTTP surface. It exists for offline development against a deterministic peer; it is **not** a primary run mode and does not support self-play (the stub's user model is invite-based, not key-based).

```sh
# Terminal A — local stub (listens on 127.0.0.1:3000)
just run-arena

# Terminal B — runtime
just run-runtime

# Terminal C — trigger PSI against the stub
curl -s -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  -d '{"game":"psi","arena_url":"http://127.0.0.1:3000"}'
```

The stub runs the full PSI hash exchange against a scripted peer. See `arena-stub/src/lib.rs` for the endpoints (`/api/v1/challenges/psi`, `/api/v1/arena/join`, `/api/v1/chat/send`, `/api/v1/chat/sync`, `/api/v1/arena/message`, `/reset`). No signing or session keys; the invite code is the agent's identity. The stub binds IPv4 only — use `127.0.0.1`, not `localhost`.

**Outbound HTTP from the guest:** Prefer **`http://127.0.0.1:3000`** in `arena_url`. The stub listens on **IPv4 only**; using `localhost` can resolve to **`::1`**, so the request never hits port 3000. The guest normalizes `localhost` to `127.0.0.1` before calling the arena. If you use a system **`HTTP_PROXY`**, set **`NO_PROXY`** so `127.0.0.1` and `localhost` are reached directly (the `just run-runtime` recipe sets this).
