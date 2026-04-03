# aria-poc-2 Just recipes

## Run from repo root (`just --list`, `just precommit`; <https://github.com/casey/just>)

## Full pre-commit — see `.cursor/rules/workflow.mdc` (README, deps, then precommit)

default:
    @just --list

## Format Rust sources

fmt:
    cargo fmt

## Lint with Clippy (`--workspace` matches the single root package)

lint:
    cargo clippy --workspace

## Run tests

test:
    cargo test

## Build

build:
    cargo build

## Full check (step 3 of pre-commit in `.cursor/rules/workflow.mdc`)

verify: fmt lint build test

## Automated checks after README + dependency-direction review

precommit: verify
