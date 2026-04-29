# Arena

This repository is a **Cargo workspace** for [Scaling Trust Arena](https://arena.nicolaos.org/) peer-to-peer agent demos. The workspace root has **no** `[package]` — only member crates.

## Architecture

The workspace uses a **hexagonal** (ports-and-adapters) shape: **`verity-core`** is the **inner hexagon**—domain types and **ports** only (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`, etc.), with no Omnia or WASI dependencies. Arena traffic is modeled as the named **`"arena"`** tool in the registry, not a separate core trait. **`verity-tools`** holds pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`) built on **`verity-core`**’s `Tool` trait. **`secure-agent`** is the **application**: it assembles a tool registry and is built as the WASM guest. **`verity-runtime`** is **infrastructure** on the host: it loads the guest `.wasm` and wires Omnia/WASI **adapters** (vault, HTTP, keyvalue, telemetry); it intentionally does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`.

| Directory | Crate | Role |
| --- | --- | --- |
| `core/` | `verity-core` | Shared domain types, trait ports (`Tool`, `ToolRegistry`, `Llm`, `Environment`, `Game`), game logic, and tests. |
| `tools/` | `verity-tools` | Pluggable tool implementations (`SecretsTool`, `HttpClientTool`, `ArenaClientTool`). Each tool is a named, auditable capability exposed through the `Tool` trait defined in `verity-core`. |
| `secure-agent/` | `secure-agent` | **WASI guest** (`wasm32-wasip2` cdylib): example agent that assembles a tool registry and plays arena games inside the sandbox. |
| `runtime/` | `verity-runtime` | **Omnia host** binary: loads the guest `.wasm`, links vault + HTTP + OpenTelemetry + **in-memory `wasi:keyvalue`** (`KeyValueDefault`; required because the guest’s HTTP stack imports keyvalue). Includes **`verity-signer`**: standalone localhost HTTP servers (Ed25519 signing for production Arena **`/play`** with **`invite`**; PKCS#8 key files at workspace root — see Production Arena section below). |

**Dependency direction:** `verity-core` has no dependency on other members. `verity-tools` depends on `verity-core`. `secure-agent` depends on `verity-core` and `verity-tools`. `verity-runtime` does **not** depend on `verity-core`, `verity-tools`, or `secure-agent`.

### Tool model

The agent receives a [`ToolRegistry`](core/src/tool.rs) at construction containing the tools it is allowed to use. Each tool has a name, description, and `execute` method taking structured JSON input and returning structured JSON output. The registry is populated at construction and treated as fixed afterward. Synchronous game logic (`play_game` in `verity-core`) dispatches the `"arena"` tool through this registry; failures surface as [`PlayGameError::Tool`](core/src/game_loop.rs). The WASI guest also registers an arena client tool so the capability set is explicit and auditable, while async game loops continue to call the arena adapter’s **`send_async`** directly to avoid deadlocks.

The former **`Runtime`** trait in `verity-core` (a fixed menu of `get_secret`, `post_json`, `create_transport`) has been removed in favor of the tool registry. The **concept** of the secure runtime—WASI sandboxing, capability scoping, the enforcement boundary—is unchanged; the `verity-runtime` host binary and adapters are the same.

The guest uses Axum’s `IntoResponse` for HTTP handlers instead of `omnia_sdk::HttpResult`: `omnia-sdk` 0.30.0 does not currently compile on this toolchain, while the WASI/Omnia crates used for vault and HTTP do.

### How it works

1. The runtime loads the secure-agent WASM and grants vault + HTTP + keyvalue (+ otel) capabilities.
2. A `POST /play` request to the runtime’s HTTP port triggers the guest’s handler, which runs **PSI only** (`"game": "psi"`). The JSON body must include **`"arena_url"`**, **`"game"`**, and **`"invite"`** (production Arena only). The guest uses **signed join** against the Arena: it calls the host **`verity-signer`** service for Ed25519 bytes, then **`POST .../arena/join`** and bearer **`sessionKey`** for chat and operator channels. After joining, the guest fetches each player’s **private number set** from **operator** messages (`GET .../arena/sync`), runs the chat protocol with the peer, computes the **guess** deterministically from the transcript and private set (not from the LLM), and submits it with **`POST .../arena/message`** (`messageType`: **`guess`**). Optional **`"signer_url"`** selects the signer (default **`http://127.0.0.1:8090`**). Optional **`"role"`** and **`"username"`** configure mover order and display name (see Production Arena). The Omnia stack listens on **`0.0.0.0:8080`** by default (override with env **`HTTP_ADDR`**, e.g. `127.0.0.1:8080`). Use **`http://127.0.0.1:8080/play`** in `curl`, not port 8000.
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

All runs use the production Arena engine (e.g. **`https://arena-engine.nicolaos.org`**). Self-play uses two invites from one challenge; a single agent uses one invite while an external peer uses the other.

### One-time setup: signing keys

Each agent identity is an Ed25519 keypair. User IDs on the Arena are derived from public keys, so two distinct keys = two distinct users. Generate both at the workspace root:

```sh
openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > arena_signing_key_1.hex
openssl genpkey -algorithm Ed25519 -outform DER | xxd -p | tr -d '\n' > arena_signing_key_2.hex
```

Each file is a single-line lowercase hex string of PKCS#8 DER. Do not commit them. Back them up if you want stable agent identities across machines. The signer reads its assigned key file at startup and exits with a clear error if it is missing, empty, or malformed; it does not generate keys.

### Get an invite pair

Create a fresh PSI challenge — the response includes two invite codes (and **`gameState.intersectionSet`**, the ground-truth intersection for honest play):

```sh
curl -sS -X POST https://arena-engine.nicolaos.org/api/v1/challenges/psi
# {
#   "id": "challenge_...",
#   "invites": ["inv_AAA...", "inv_BBB..."],
#   "gameState": { "intersectionSet": [ ... ] }
# }
```

Save both invites — the first goes to your first agent, the second to your second agent (or to whoever is playing the peer side).

### Game flow (PSI)

Roughly:

1. Both agents join with their invite and session key; the operator delivers a private number set to each player (read via arena operator sync).
2. **First mover** sends **`Hello.`** on chat; **second mover** responds and they agree to play and to a hash-based PSI strategy.
3. **Second mover** sends a JSON array of SHA-256 hex hashes of their private numbers; **first mover** maps matching hashes to plaintext and replies with a JSON array of intersecting numbers.
4. **Second mover** checks those numbers against their private set, reports **Result: correct/incorrect**, and both sides say goodbye on chat.
5. Each guest computes the final **`guess`** array from the transcript and private set, then **`POST /api/v1/arena/message`** with **`messageType`**: **`guess`**. The operator returns scores (via operator messages).

The LLM follows the scripted dialogue; the **submitted guess is computed in code**, not taken from model output.

### Self-play (`missionary` vs `friend`)

Three terminals:

```sh
# Terminal A — runtime (shared)
just run-runtime

# Terminal B — first agent
./scripts/play.sh missionary inv_AAA

# Terminal C — second agent
./scripts/play.sh friend inv_BBB
```

Each `play.sh` invocation owns its own signer (started in background, killed on script exit). Start B and C within a few seconds of each other. Both will print interleaved transcript lines as the game progresses, then a guess submission, then the operator’s score message.

**`scripts/play.sh` assumes the runtime is already running.** It performs one `curl` to **`http://127.0.0.1:8080/play`**.

### Single agent vs external peer

When playing against someone else’s agent (a teammate, a random opponent via the Arena’s matchmaking, etc.), run **one** invocation of `scripts/play.sh`:

```sh
just run-runtime                              # terminal A
./scripts/play.sh missionary inv_AAA          # terminal B
```

The peer side joins with the other invite via whatever client they use.

### Manual `/play` (without `play.sh`)

You can run **`verity-signer`** yourself (see **`just run-signer-1`** / **`just run-signer-2`**) and call **`curl`**:

```sh
curl -sS -X POST http://127.0.0.1:8080/play \
  -H "Content-Type: application/json" \
  --max-time 600 \
  -d '{"game":"psi","arena_url":"https://arena-engine.nicolaos.org","invite":"inv_AAA"}'
```

The request can take **minutes**: each turn calls the LLM and the arena over WASI HTTP. **`curl -s`** prints nothing until the response is ready — watch the **runtime** terminal for transcript lines (`peer <-` / `SecureAgent ->`). **`verity-signer`** reads the key file; the guest never touches the signing key.

### `/play` body fields

| Field        | Required | Purpose                                                                 |
|--------------|----------|-------------------------------------------------------------------------|
| `game`       | yes      | Must be `"psi"`.                                                        |
| `arena_url`  | yes      | Arena base URL (e.g. production engine URL).                         |
| `invite`     | yes      | Invite from **`POST /api/v1/challenges/psi`**.                         |
| `signer_url` | no       | Default `http://127.0.0.1:8090`. Use port **8091** for the second key. |
| `role`       | no       | `"first"` or `"second"`. Default `"second"`. Needed for self-play order. |
| `username`   | no       | Display name on the Arena. Default `"missionary"`.                    |

### If something goes wrong

- **Operator sync returns no parseable private_set:** run with **`RUST_LOG=debug`** on the runtime. Look for `operator_sync returned N operator messages`. The guest parses JSON arrays/objects (see `parse_private_set` in `secure-agent/src/operator_parse.rs`) and prose with a brace list, e.g. `Your private set is: {277, 322, ...}.`
- **Guess rejected (HTTP 400):** read the error body; confirm payload shape for **`POST /api/v1/arena/message`** against Arena docs.
- **Hang in chat sync:** the other agent may not be running or never sent on the channel.
- **Empty score after submit:** both agents may need to submit before the operator emits final scores; use debug logs on post-submit operator sync.

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
