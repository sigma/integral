# integral-cli/src/commands

## Overview

One module per `integral` subcommand. A command parses its arguments, does
the I/O (MIDI ports, files, network, prompts) and prints the result. The
protocol, codec and state-machine logic stays in `integral-core`
(`sysex`, `librarian`, `wireless`, `waves`, `svd_convert`). Most commands
talk to the device through `IntegraDevice` or `midi::open_midi`
([`../README.md`](../README.md)). `wave`, `pack list` and `pack check` work
offline.

## Interface

- `mod.rs` declares the submodules. `bridge` exists only on Unix
  (`#[cfg(unix)]`).
- `ping.rs`: `ping(port_pattern, timeout, quiet) -> Result<u8>` sends an
  Identity Request and returns the device ID.
- `dump.rs`: `read_param` reads a named parameter with RQ1 (Studio Set name,
  master level, Part mixer, tone name, and `tone-waves` through
  `tone_waves::read`). `write_param` sends one DT1.
- `raw.rs`: `raw_read`, `raw_rq1_multi`, `raw_send` (DT1) and `send_raw_hex`.
- `monitor.rs`: `monitor(port_pattern, bridge)` prints incoming MIDI as hex
  and decodes DT1s. With a bridge port it also forwards traffic both ways.
- `bridge.rs`: `bridge(&BridgeArgs)` launches the Device service
  (`integral-device-service`) on the link `--link` names: `wireless` (RDDP
  discovery or `--host`; also shares the port `INTEGRA-7 (Wireless)`), `rtp`
  (`--port` names the session) or `usb`. It opens the note port
  `Integral Device Service` and runs until killed.
- `pack.rs`:
  - `list` reads a Pack file.
  - `export` reads User slots, and with `--current-part` / `--current-set`
    the Temporary Part tone and Studio Set, with RQ1, and writes an SVD.
  - Shared helpers: `read_pack`, `Throttle` and `run_export`.
  - Argument types: `UserSlots`, `type_parser` and `ListFormat`.
- `pack_import.rs`: `import` and `reinit`, configured by `ImportOptions` and
  `ReinitOptions`. `EXIT_PARTIAL = 2`.
- `pack_check.rs`: `run(paths, top, format)` reads files and directories
  offline and totals their Conversion reports.
- `expansion.rs`:
  - `list` and `load(..., dry_run, yes)` are the `expansion` subcommands.
  - `parse_assignments` and `stage` build the new slot layout, and
    `read_slots` reads the current one.
  - `render_list`, `render_plan` and `slots_line` print text or JSON.
  - `EXIT_LOAD_FAILED = 2`, `OFF_NAME = "OFF"`.
  - `OutputFormat`, the `--format` type shared with `wave list`,
    `pack check` and `pack import`.
- `svd.rs`: `svd_validate` decodes one SN-S entry from an SVD file and
  compares it, byte by byte, with the Part tone read from the device.
- `tone_waves.rs`: `read(dev, part)` names the Waves used by a Part's
  Temporary tone (PCM Synth, PCM Drum, SN Synth). The line builders
  `pcm_synth_lines`, `pcm_drum_lines`, `sn_synth_lines` and `no_waves_line`
  are public.
- `wave.rs`: `list`, `parse_group`, `format_number`, `render_list` and
  `Listing` browse the Wave catalog offline.

## Invariants

- `pack import`:
  - Planning only reads from the device. `store` is the only function that
    writes, and `--dry-run` returns before it.
  - Before storing over used slots, an undo SVD is exported (`import`: when
    the plan overwrites slots; `reinit`: when a picked slot is in use). It is named
    `UNDO-<stem>-<unix time>.SVD`, or `UNDO-reinit-<type>-<unix time>.SVD`
    for `reinit`. If the undo path already exists the command fails.
    `--undo-file` conflicts with `--no-undo`.
  - `--apply-effects` only applies when a single entry is loaded into
    Temporary.
- `pack export` refuses to overwrite an existing output file unless `--force`
  is given. It fails if nothing is selected.
- Confirmations (overwrites, an Expansion load that silences Parts) prompt
  `y/N` only on a TTY. Otherwise they refuse unless `--yes` is given.
- Exit codes:
  - `import` and `reinit`: 0 means everything was stored, 1 means the device
    was untouched, and 2 means a partial store.
  - `expansion load`: 2 means the load was sent but failed. An error (1)
    means nothing was sent.
  - `pack check`: 1 means a path could not be read. Files that `Pack::read`
    rejects are counted, not treated as errors.
- `parse_assignments` rejects the same Expansion in two slots. ExPCM takes
  every slot.
- `raw_read` fails if any requested byte is missing from the replies.
- `raw-send` and `raw-hex` refuse input over one message (`MAX_DT1_DATA`
  data bytes, `MAX_MESSAGE_LEN` bytes) before opening a port.
- `wave` and `pack check` never open a MIDI port.
- DT1 bursts go through `Throttle`, which uses `DeviceState`'s raw queue so
  that writes are not merged.

## Side Effects

- MIDI out:
  - Identity Request.
  - RQ1 reads: `dump`, `raw`, `svd`, `tone_waves`, `expansion`,
    `pack export`, and the `pack import` slot-name reads.
  - DT1 writes: `write`, `raw-send`, `expansion load`, `pack import` and
    `pack reinit`.
  - Raw hex SysEx.
- MIDI in: replies are read with a deadline. If the input disconnects the
  command fails.
- Virtual MIDI ports and network (`bridge`): the Device service opens the
  note port, and for Wireless an RDDP UDP broadcast, then a TCP session to the
  device.
- Disk: reads Pack, SVD and directory trees. Writes the export SVD and the
  undo SVD.
- `stdin` prompts. Results and JSON go to `stdout`, and progress and
  warnings go to `stderr`.

## Verification

- Run: `cargo test -p integral-cli`
