# Integral — Integra-7 control surface
# Run `just` to see all available targets.

set dotenv-load := false

# default: list available targets
default:
    @just --list

# format all Rust code
fmt:
    cargo fmt --all

# check formatting without modifying files
fmt-check:
    cargo fmt --all --check

# run clippy on all targets
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# build all crates (native)
build:
    cargo build --workspace

# build WASM targets
build-wasm:
    cargo build --target wasm32-unknown-unknown -p integral-core -p integral-wasm

# run all tests
test:
    cargo test --workspace

# format, lint, build, and test — the full pre-commit check
check: fmt-check lint build build-wasm test

# clean build artifacts
clean:
    cargo clean

# ping the INTEGRA-7 device
ping *ARGS:
    python3 scripts/ping-device.py {{ARGS}}

# watch for changes and rebuild
watch:
    cargo watch -c -x 'check --workspace'
