# Integral — Integra-7 control surface
# Run `just` to see all available targets.

set dotenv-load := false

# default: list available targets
default:
    @just --list

# format all code (Rust + Nix)
fmt:
    cargo fmt --all
    nixfmt *.nix

# check formatting without modifying files
fmt-check:
    cargo fmt --all --check
    nixfmt --check *.nix

# run clippy on all targets
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# typecheck the web app
lint-web:
    cd web && npx tsc -b

# build all crates (native)
build:
    cargo build --workspace

# build WASM targets (check only)
build-wasm:
    cargo build --target wasm32-unknown-unknown -p integral-core -p integral-wasm

# build WASM package for web consumption
pack-wasm:
    wasm-pack build crates/integral-wasm --target web --out-dir ../../web/pkg

# run all tests
test:
    cargo test --workspace

# build the web app for production (includes WASM pack)
build-web: pack-wasm
    cd web && npx vite build

# format, lint, build, and test — the full pre-commit check
check: fmt-check lint lint-web build build-wasm test

# clean build artifacts
clean:
    cargo clean
    rm -rf web/dist web/pkg

# ping the INTEGRA-7 device
ping *ARGS:
    cargo run -p integral-cli -- ping {{ARGS}}

# start the web dev server
dev-web: pack-wasm
    cd web && npx vite

# watch for changes and rebuild
watch:
    cargo watch -c -x 'check --workspace'
