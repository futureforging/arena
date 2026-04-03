# aria-poc-2 Just recipes — Cargo workspace

## Run from repo root (`just --list`, `just precommit`; <https://github.com/casey/just>)

## Full pre-commit — see `.cursor/rules/workflow.mdc` (README, deps, then precommit)

default:
    @just --list

## Format all workspace members

fmt:
    cargo fmt --all

## Lint all workspace members

lint:
    cargo clippy --workspace

## Run tests (all workspace members)

test:
    cargo test --workspace

## Run arena-stub unit tests only (knock-knock script + invitation reset)

test-arena:
    cargo test -p arena-stub

## Build all workspace members

build:
    cargo build --workspace

## Full check

verify: fmt lint build test

## Automated checks after README + dependency-direction review

precommit: verify

## Run the main agent (knock-knock demo via HTTP to arena-stub on 127.0.0.1:3000)

run-agent:
    cargo run -p aria-poc-2

## Run the full knock-knock demo (arena-stub + agent).
## Starts arena-stub in background, runs agent, then stops arena-stub.

demo:
    @echo "Starting arena-stub..."
    @cargo run -p arena-stub &
    @sleep 1
    @echo "Running agent..."
    @cargo run -p aria-poc-2
    @echo "Demo complete."

## Run the arena stub (HTTP audience server on 127.0.0.1:3000)

run-arena:
    cargo run -p arena-stub
