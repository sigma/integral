# Integral — Integra-7 Control Surface

## Project Overview

Integral is a cross-platform, open-source control surface for the Roland
INTEGRA-7 Synthesizer Module. It provides a web UI (via WASM) and a VST3
plugin, both backed by a shared Rust core library.

## Repository Layout

```
crates/
  integral-core/    # Portable Rust library: SysEx engine, state, address maps
  integral-wasm/    # WASM bindings for the web frontend
  integral-vst/     # VST3 wrapper via nih-plug
  integral-cli/     # CLI tools (device ping, etc.)
web/                # TypeScript / React frontend
docs/               # Design docs, PRD, MIDI reference notes
  midi/             # INTEGRA-7 MIDI Implementation (see docs/midi/README.md)
```

## Version Control (jj)

- **Never use raw git commands.** This is a jj-managed repo.
- **One logical change per commit.** Keep commits atomic and reviewable.
- **Describe before coding:** `jj describe -m "type(scope): message"` first.
- **Always use `-m` flags.** Never open an editor.
- After creating or renaming files, run `jj status` before any nix command.

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[(scope)]: <description>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`, `perf`,
`build`, `style`. Lowercase, imperative mood, no trailing period.

Include the beadwork ticket ID (e.g. `i7-xyz`) on a separate line in the
body when applicable.

## Task Tracking (beadwork)

- Run `bw prime` at the start of every session.
- Create tickets before starting non-trivial work.
- One ticket per worktree. `bw start <id>` → work → commit → `bw close <id>` → `bw sync`.
- Reference ticket IDs in commit messages.

## Architecture Principles

### Device specification, not hardcoded data

Device-specific constants (output names, FX types, room types, tone banks,
part count, parameter ranges) must **not** be hardcoded in UI views or CLI
commands. They belong in a declarative **device specification layer** in
`integral-core` (e.g. `param_registry`, tone bank definitions, device
capabilities). Views and commands consume the spec; they don't define it.

When adding a new UI panel or CLI command, ask: *"Would this break if the
device had different outputs/parts/FX types?"* If yes, the data needs to
come from the spec layer.

### Protocol abstraction

Callers should never construct raw SysEx bytes or manipulate address offsets
directly. Use the typed helpers in `integral-core`:

- `DeviceState` for state management and queued sends
- `params::` module for address computation
- `sysex::` module for message construction
- High-level accessors (e.g. `set_part_level`, `change_part_tone`) over
  raw `send_dt1`

The device ID (`0x10` default) must be **discoverable** from the SysEx
Identity Reply, not assumed. Any new code that references a device ID
should accept it as a parameter, not hardcode it.

### Module boundaries

- **One concern per module.** A crate's `main.rs` should contain only
  command definitions and dispatch. Helpers, parsers, and protocol logic
  belong in separate modules or crates.
- **SVD file operations** belong in `integral-core` (or a dedicated crate),
  not in the CLI.
- **Parse/format helpers** (hex parsing, byte formatting) should be
  consolidated into shared utility modules, not duplicated.
- **Remove scaffolding code.** Trial-and-error constructs from
  reverse-engineering (e.g. scan/brute-force commands) should be removed
  or moved to a `dev-tools` module once their purpose is served.

### Consistent project structure

- All Rust crates live under `crates/`. External patches (e.g. baseview)
  also belong under `crates/patches/` for consistency.
- Third-party vendored code should be clearly separated from project code.

## Rust Conventions

- Edition: 2024.
- Use `thiserror` for library errors, `anyhow` for binaries/tests.
- Prefer strong typing over raw bytes — wrap SysEx addresses, checksums,
  parameter values in newtypes.
- All public API items must have doc comments.
- Run `cargo clippy -- -D warnings` and `cargo fmt --check` before
  committing. The nix dev shell provides these tools.
- Target `wasm32-unknown-unknown` must always compile for `integral-core`.

## Nix

- The `flake.nix` defines the dev environment. Enter with `nix develop`.
- Never install tooling globally — add it to the flake's `devShells`.
- Pin all inputs via `flake.lock`.

## MIDI Implementation Reference

The complete INTEGRA-7 MIDI Implementation (v1.00) is extracted into
`docs/midi/`. **Always consult these docs** when implementing SysEx
parameters, address maps, or bank select logic. Key files:

- `docs/midi/01-protocol.md` — DT1/RQ1 message format, checksum algorithm
- `docs/midi/04-address-map.md` — Top-level address map, Setup, System
- `docs/midi/05-studio-set.md` — Studio Set parameters (all sub-blocks)
- `docs/midi/06-pcm-synth-tone.md` — PCM Synth Tone (Common, MFX, Partials)
- `docs/midi/03-bank-select-tables.md` — Bank Select MSB/LSB/PC for all types

## SysEx Engine Rules

These are critical correctness rules derived from the Roland INTEGRA-7 MIDI
Implementation document:

1. **Checksum:** `sum = address_bytes + data_bytes; checksum = (128 - (sum % 128)) % 128`.
2. **Packet limit:** Never exceed 256 bytes per SysEx message.
3. **Throttle:** Minimum 20 ms between consecutive SysEx transmissions.
4. **Nibblized data:** Multi-byte parameters use 7-bit encoding (one value
   per byte, high bit always 0).
5. **Model ID:** Always `00 00 64` for the Integra-7.
6. **DT1 header:** `F0 41 <dev_id> 00 00 64 12 <addr> <data> <checksum> F7`.
7. **RQ1 header:** `F0 41 <dev_id> 00 00 64 11 <addr> <size> <checksum> F7`.

## Frontend Conventions

- TypeScript strict mode, no `any`.
- React functional components with hooks.
- State flows from the Rust/WASM core — the React layer is a thin view.

## Just Targets

The `justfile` provides standard targets. Run `just` to list them.

| Target | Purpose |
|--------|---------|
| `just fmt` | Format all code (Rust + Nix) |
| `just fmt-check` | Check formatting (no changes) |
| `just lint` | Clippy with `-D warnings` |
| `just build` | Build all crates (native) |
| `just build-wasm` | Build WASM targets |
| `just test` | Run all tests |
| `just check` | Full pre-commit: fmt-check + lint + build + build-wasm + test |
| `just clean` | Remove build artifacts |
| `just ping` | Ping the INTEGRA-7 device |
| `just run` | Run standalone VST binary (native app) |
| `just dev-web` | Start web dev server (pack WASM + Vite) |

## Agent Behavior

- **Read before editing.** Never modify code you haven't read.
- **Granular commits.** Each logical change gets its own jj commit.
- **No speculative features.** Only build what's requested.
- **Run `just check` before completing any Rust change.** This runs
  fmt-check, lint, build (native + WASM), and tests. Do not commit
  if any target fails.
- **Ping the device.** At the start of MIDI-related work sessions, run
  `just ping` (which runs `cargo run -p integral-cli -- ping`) to verify
  the Integra-7 is reachable. Fail fast if the device is offline.
- **Keep CLAUDE.md current.** Update this file when conventions change.
