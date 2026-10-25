# integral-cli

## Overview

The `integral` command-line tool for the INTEGRA-7. It covers several jobs:

- Device diagnostics: ping, read and write parameters, raw SysEx, and a MIDI
  monitor.
- The Wireless bridge (Unix only).
- The Librarian: list, check, export, import and reinit Packs.
- Expansion slot management and Wave catalog browsing.

It depends on `integral-core` with the `serde` and `smf` features, and uses
`midir` for MIDI, `clap` for arguments and `anyhow` for errors.

## Subdirectories & Boundaries

- src: [`./src/README.md`](./src/README.md) - the binary entry point
  (`main.rs`), the device handle and the MIDI and hex helpers.
- src/commands: [`./src/commands/README.md`](./src/commands/README.md) - one
  module per subcommand, covering its device, file and network I/O.
- `tests/` holds integration tests that run the binary offline
  (`pack_check.rs`, which uses the `integral-core` fixtures, and
  `wave_list.rs`).
  These tests are not documented as a package.

## Verification

- Run: `cargo test -p integral-cli`
- Run: `just ping` (needs the device)
