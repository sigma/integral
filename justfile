# Integral — Integra-7 control surface
# Run `just` to see all available targets.

set dotenv-load := false

# the plugin bundle `bundle-plugin` assembles under target/bundle (and its executable)
bundle_name := "Integral"

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

# install web dependencies when missing or older than package-lock.json
web-deps:
    cd web && if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then npm ci; fi

# typecheck the web app (needs the WASM package and web dependencies)
lint-web: pack-wasm web-deps
    cd web && npx tsc -b

# run the web component tests (against the packed WASM)
test-web: pack-wasm web-deps
    cd web && npx vitest run

# lint the web CSS modules
lint-css: web-deps
    cd web && npx stylelint "src/**/*.css"

# build the web app for both bases and check its PWA manifest and precache
check-pwa: pack-wasm web-deps
    cd web && npx vite build --logLevel warn --outDir dist/check-root --emptyOutDir && node scripts/check-pwa-build.mjs dist/check-root /
    cd web && GITHUB_PAGES=true npx vite build --logLevel warn --outDir dist/check-pages --emptyOutDir && node scripts/check-pwa-build.mjs dist/check-pages /integral/

# build the web app the plugin embeds (web/dist-plugin: no service worker, base /) and check it
build-web-plugin: pack-wasm web-deps
    cd web && npx vite build --mode plugin --logLevel warn --emptyOutDir && node scripts/check-plugin-build.mjs dist-plugin

# build the plugin (VST3/CLAP library and integral-app) with its web UI embedded
build-plugin: build-web-plugin
    cargo build -p integral-vst --release

# assemble target/bundle/Integral.vst3 from a fresh plugin build, ad-hoc signed (macOS)
[macos]
bundle-plugin: build-plugin
    #!/usr/bin/env bash
    set -euo pipefail
    # The cargo target dir may be shared between workspaces, so copy the
    # dylib straight after the build, before another build replaces it.
    meta=$(cargo metadata --format-version 1 --no-deps)
    target=$(jq -r .target_directory <<<"$meta")
    version=$(jq -r '.packages[] | select(.name == "integral-vst") | .version' <<<"$meta")
    bundle="{{justfile_directory()}}/target/bundle/{{bundle_name}}.vst3"
    rm -rf "$bundle"
    mkdir -p "$bundle/Contents/MacOS"
    cp "$target/release/libintegral_vst.dylib" "$bundle/Contents/MacOS/{{bundle_name}}"
    printf 'BNDL????' > "$bundle/Contents/PkgInfo"
    cat > "$bundle/Contents/Info.plist" <<PLIST
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
      <key>CFBundleDevelopmentRegion</key><string>en</string>
      <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
      <key>CFBundleExecutable</key><string>{{bundle_name}}</string>
      <key>CFBundleIdentifier</key><string>com.sigma.integral.vst3</string>
      <key>CFBundleName</key><string>{{bundle_name}}</string>
      <key>CFBundlePackageType</key><string>BNDL</string>
      <key>CFBundleSignature</key><string>????</string>
      <key>CFBundleShortVersionString</key><string>$version</string>
      <key>CFBundleVersion</key><string>$version</string>
    </dict>
    </plist>
    PLIST
    codesign --force -s - "$bundle"
    echo "$bundle"

# run a test-host scenario (e.g. drag) against the bundled plugin; JSON lines on stdout (macOS)
[macos]
[positional-arguments]
plugin-test scenario *ARGS: bundle-plugin
    cargo run -q -p integral-plugin-host -- --bundle "target/bundle/{{bundle_name}}.vst3" --out-dir target/plugin-test "$@"

# render the PWA icons from web/public/icon.svg (librsvg from the pinned nixpkgs)
web-icons:
    cd web/public && for s in 192 512; do nix shell --quiet --inputs-from ../.. nixpkgs#librsvg -c rsvg-convert -w $s -h $s icon.svg -o pwa-$s.png; done

# build all crates (native)
build:
    cargo build --workspace

# build WASM targets (check only)
build-wasm:
    cargo build --target wasm32-unknown-unknown -p integral-core -p integral-wasm

# build WASM package for web consumption
pack-wasm:
    wasm-pack build crates/integral-wasm --target web --out-dir ../../target/wasm-pkg
    # Copy only the files that changed: a running dev server fully reloads on any write to web/pkg.
    mkdir -p web/pkg && for f in target/wasm-pkg/*; do cmp -s "$f" "web/pkg/${f##*/}" || cp "$f" web/pkg/; done

# run all tests
test:
    cargo test --workspace

# rewrite the host parameter list snapshots (review the diff: it moves saved automation)
regen-host-params:
    cargo run -p integral-core --example regen_host_params

# build the web app for production (includes WASM pack)
build-web: pack-wasm
    cd web && npx vite build

# format, lint, build, and test — the full pre-commit check
check: fmt-check lint lint-web lint-css build-web-plugin build build-wasm test test-web check-pwa

# clean build artifacts
clean:
    cargo clean
    rm -rf web/dist web/dist-plugin web/pkg

# ping the INTEGRA-7 device
ping *ARGS:
    cargo run -p integral-cli -- ping {{ARGS}}

# run the Device service on a link: --link wireless (default), rtp --port <name>, or usb
bridge *ARGS:
    cargo run -p integral-cli -- bridge {{ARGS}}

# monitor the MIDI traffic
monitor *ARGS:
    cargo run -p integral-cli -- monitor {{ARGS}}

# run the standalone VST binary (native app with MIDI), web UI embedded
run *ARGS: build-web-plugin
    cargo run -p integral-vst --bin integral-app --release -- {{ARGS}}

# start the web dev server
dev-web: pack-wasm
    cd web && npx vite

# watch for changes and rebuild
watch:
    cargo watch -c -x 'check --workspace'
