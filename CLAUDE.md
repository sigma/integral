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
web/                # TypeScript / React frontend
scripts/            # Utility scripts (device ping, etc.)
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

## Agent Behavior

- **Read before editing.** Never modify code you haven't read.
- **Granular commits.** Each logical change gets its own jj commit.
- **No speculative features.** Only build what's requested.
- **Verify builds.** Run `nix develop --command cargo check` after Rust changes.
- **Test after changes.** Run `nix develop --command cargo test` when tests exist.
- **Ping the device.** At the start of MIDI-related work sessions, run
  `nix develop --command python3 scripts/ping-device.py` to verify the
  Integra-7 is reachable. Fail fast if the device is offline.
- **Keep CLAUDE.md current.** Update this file when conventions change.
