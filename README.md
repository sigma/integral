# OpenLink — Integra-7 Control Surface

A cross-platform, open-source control surface for the Roland INTEGRA-7
Synthesizer Module.

## Targets

- **Web** — WASM + React app using Web MIDI / RTP MIDI
- **VST3** — DAW plugin via nih-plug

Both share a common Rust core that handles SysEx construction, checksum
calculation, state management, and the full Integra-7 address map.

## Development

```bash
nix develop   # enter the dev shell (Rust 1.94, wasm-pack, Node)
cargo check   # verify the workspace compiles
cargo test    # run tests
```

## Project Structure

| Path | Description |
|------|-------------|
| `crates/openlink-core/` | Portable Rust library: SysEx engine, state, address maps |
| `crates/openlink-wasm/` | WASM bindings for the web frontend |
| `crates/openlink-vst/` | VST3 wrapper via nih-plug |
| `web/` | TypeScript / React frontend (planned) |

## License

MIT
