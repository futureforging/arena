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
cargo build -p arena-guest --target wasm32-wasip2 --release
cargo build -p arena-runtime --release

# Run (WASM path may be target/wasm32-wasip2/release/arena_guest.wasm)
./target/release/arena-runtime run ./target/wasm32-wasip2/release/arena_guest.wasm
```

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
