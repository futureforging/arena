# arena — Just recipes — Cargo workspace

default:
    @just --list

## Format all workspace members

fmt:
    cargo fmt --all

## Lint all workspace members (native target only)

lint:
    cargo clippy --workspace

## Run tests (all workspace members, native target only)

test:
    cargo test --workspace

## Build runtime (native)

build-host:
    cargo build -p verity-runtime

## Build secure-agent (wasm32-wasip2)

build-guest:
    cargo build -p secure-agent --target wasm32-wasip2

## Build everything

build: build-host build-guest

## Full check (native targets — guest checked separately)

verify: fmt lint build test

## Automated checks

precommit: verify

## Run the production-arena signer service (port 8090; reads arena_signing_key.hex from workspace root)

run-signer:
    cargo run -p verity-runtime --bin verity-signer

## Run the arena stub

run-arena:
    cargo run -p arena-stub

## Run the runtime with the secure-agent guest (HTTP on 0.0.0.0:8080 unless HTTP_ADDR is set; curl POST /play)

run-runtime: build-guest
    NO_PROXY=127.0.0.1,localhost,::1 cargo run -p verity-runtime --bin verity-runtime -- run target/wasm32-wasip2/debug/secure_agent.wasm

## Run core tests only

test-core:
    cargo test -p verity-core

## Run arena-stub tests only

test-arena:
    cargo test -p arena-stub
