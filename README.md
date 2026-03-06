# Arena

Omnia-based Docker runtime with an HTTP echo handler. The guest runs as a WebAssembly component inside a sandboxed Omnia runtime.

## Quick Start

```bash
# Build and run
docker compose up --build

# In another terminal, test the echo endpoint
curl -X POST http://localhost:8080/any/path -H "X-Custom: foo" -d '{"hello":"world"}'

# Stop
docker compose down
```

## Local Development

```bash
# Build guest (WASM) and runtime
cargo build -p arena-guest --target wasm32-wasip2
cargo build -p arena-runtime

# Run (WASM path: target/wasm32-wasip2/debug/arena_guest.wasm)
./target/debug/arena-runtime run ./target/wasm32-wasip2/debug/arena_guest.wasm
```

## Integration Test

The echo handler has an integration test that spawns the runtime, sends a request, and asserts the echoed response. It enforces a **15-second max duration** to fail fast.

```bash
# Build artifacts first, then run the test
cargo build -p arena-guest --target wasm32-wasip2
cargo build -p arena-runtime

cargo test -p arena-runtime --test echo_integration
```

- **When artifacts exist**: Test runs in ~2–5 seconds (server startup + HTTP round-trip).
- **When artifacts are missing**: Test fails in &lt;1 second with a clear build instruction.
- **On timeout**: Test fails after 15 seconds with a timeout message.

## Project Structure

- `guest/` - WASM component (echo handler) compiled to `wasm32-wasip2`
- `runtime/` - Omnia host runtime with WasiHttp + WasiOtel
- `Dockerfile` - Multi-stage build for production image
- `docker-compose.yml` - One-command run

## Echo Response

The handler returns JSON with the request details:

- `method` - HTTP method
- `path` - Request path
- `query` - Query string (or null)
- `headers` - Request headers
- `body` - Raw request body
