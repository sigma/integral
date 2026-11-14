# Integral — Integra-7 control surface
# Run `just` to see all available targets.

set dotenv-load := false

# the plugin bundle `bundle-plugin` assembles under target/bundle (and its executable)
bundle_name := "Integral"
# the Device service binary `install` puts under ~/Library/Application Support/Integral, and its LaunchAgent label
service_bin := "integral-device-service"
service_label := "com.sigma.integral.device-service"

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

# build the plugins (the VST3 library and integral-app) with their web UI embedded
build-plugin: build-web-plugin
    cargo build -p integral-vst --release

# assemble target/bundle/Integral.vst3 from a fresh plugin build, ad-hoc signed (macOS).
# One bundle exports the six plugins of the family; hosts list each class.
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

# Replaces an installed copy and restarts the agent. Stop the agent before `just bridge`
# (both claim the service socket) with the command it prints.
# install the plugins and the Device service for this user, the service as a LaunchAgent up at login (macOS)
[macos]
install: bundle-plugin
    #!/usr/bin/env bash
    set -euo pipefail
    label="{{service_label}}"
    service="{{service_bin}}"
    vst3_dir="$HOME/Library/Audio/Plug-Ins/VST3"
    # Where the service reads service.toml (config_path_in in
    # crates/integral-device-service/src/link_config.rs).
    support_dir="$HOME/Library/Application Support/Integral"
    log_dir="$HOME/Library/Logs/Integral"
    plist="$HOME/Library/LaunchAgents/$label.plist"
    domain="gui/$UID"
    # The cargo target dir may be shared between workspaces, so copy the
    # binary straight after the build, before another build replaces it.
    cargo build -p "$service" --release
    target=$(cargo metadata --format-version 1 --no-deps | jq -r .target_directory)
    mkdir -p "$support_dir" "$log_dir" "$vst3_dir" "$(dirname "$plist")"
    # A new file renamed over the old one: overwriting the binary a running
    # agent maps would invalidate its code signature and get it killed.
    cp "$target/release/$service" "$support_dir/$service.new"
    mv -f "$support_dir/$service.new" "$support_dir/$service"
    # Copied beside the old bundle first, so a failed copy leaves it installed.
    vst3="$vst3_dir/{{bundle_name}}.vst3"
    rm -rf "$vst3.new"
    cp -R "{{justfile_directory()}}/target/bundle/{{bundle_name}}.vst3" "$vst3.new"
    rm -rf "$vst3"
    mv "$vst3.new" "$vst3"
    cat > "$plist" <<PLIST
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
      <key>Label</key><string>$label</string>
      <key>ProgramArguments</key>
      <array>
        <string>$support_dir/$service</string>
      </array>
      <key>RunAtLoad</key><true/>
      <key>KeepAlive</key><true/>
      <key>StandardOutPath</key><string>$log_dir/device-service.log</string>
      <key>StandardErrorPath</key><string>$log_dir/device-service.err.log</string>
    </dict>
    </plist>
    PLIST
    plutil -lint -s "$plist"
    # Not loaded is fine (a first install, or it went away meanwhile); still
    # loaded afterwards is not. Bootstrapping a label launchd is still
    # tearing down fails.
    if launchctl print "$domain/$label" >/dev/null 2>&1; then
        launchctl bootout "$domain/$label" || true
        for _ in $(seq 50); do
            launchctl print "$domain/$label" >/dev/null 2>&1 || break
            sleep 0.1
        done
        if launchctl print "$domain/$label" >/dev/null 2>&1; then
            echo "error: $label is still loaded 5 s after launchctl bootout" >&2
            exit 1
        fi
    fi
    launchctl bootstrap "$domain" "$plist"
    echo "Installed $vst3_dir/{{bundle_name}}.vst3"
    echo "Installed $support_dir/$service, running as the LaunchAgent $label"
    echo "Logs: $log_dir"
    echo "Stop it before \`just bridge\` (both claim the service socket):"
    echo "  launchctl bootout $domain/$label"
    echo "It starts again at the next login, or now with:"
    echo "  launchctl bootstrap $domain \"$plist\""

# run a test-host scenario (load, drag, recall, automate, binding-mismatch, kit-binding) against the bundled plugins; JSON lines on stdout (macOS).
# `load` checks every plugin class; `drag` drags a Studio Set slider (`--param`, default "Part 1 Level").
# `drag`, `recall`, `automate`, `binding-mismatch` and `kit-binding` run the Device service themselves on a fake link (or the INTEGRA-7
# with `--device rtp:<session>` / `usb`) at the socket the plugins look for; `binding-mismatch --no-editor` opens no window.
# `load` refuses to run while a Device service listens (the plugins would connect to it) unless `--allow-service`;
# the others always refuse.
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
[positional-arguments]
ping *ARGS:
    cargo run -p integral-cli -- ping "$@"

# run the Device service on a link: --link wireless (default), rtp --port <name>, or usb
[positional-arguments]
bridge *ARGS:
    cargo run -p integral-cli -- bridge "$@"

# monitor the MIDI traffic
[positional-arguments]
monitor *ARGS:
    cargo run -p integral-cli -- monitor "$@"

# run the standalone VST binary (native app with MIDI), web UI embedded
[positional-arguments]
run *ARGS: build-web-plugin
    cargo run -p integral-vst --bin integral-app --release -- "$@"

# start the web dev server
dev-web: pack-wasm
    cd web && npx vite

# watch for changes and rebuild
watch:
    cargo watch -c -x 'check --workspace'
