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
- `ping.rs`: `ping(endpoint, timeout, quiet) -> Result<u8>` sends an
  Identity Request and returns the device ID.
- `dump.rs`: `read_param` reads a named parameter with RQ1 (Studio Set name,
  master level, Part mixer, tone name, and `tone-waves` through
  `tone_waves::read`). `write_param` sends one DT1.
- `raw.rs`: `raw_read`, `raw_rq1_multi`, `raw_send` (DT1) and `send_raw_hex`.
- `monitor.rs`: `monitor(endpoint, bridge)` prints incoming MIDI as hex
  and decodes DT1s. With a bridge port it also forwards traffic both ways.
- `bridge.rs`: `bridge(&ServiceArgs)` launches the Device service through
  `ServiceArgs::run` of `integral-device-service`, the entry point the
  `integral-device-service` binary shares, on the link `--link` names:
  `wireless` (the default; RDDP discovery or `--host`; also shares the port
  `INTEGRA-7 (Wireless)`), `rtp` (`--port` names the session) or `usb`. It
  reads no `service.toml`. It opens the note port `Integral Device Service`,
  listens for clients on the Unix socket `--socket` names (default:
  `default_socket_path()` of the service crate, in a private per-user
  directory) and runs until killed.
- `pack.rs`:
  - `list` reads a Pack file.
  - `export` reads User slots, and with `--current-part` / `--current-set`
    the Temporary Part tone and Studio Set, with RQ1, and writes an SVD.
  - Shared helpers: `read_pack`, `Throttle`, `send_next` (sends a
    `DeviceState`'s next queued message once its throttle allows; `Throttle`
    and `part set` use it) and `run_export`.
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
- `init.rs`: `run(endpoint, device_id, timeout, &InitArgs) -> Result<u8>`
  resets one area to INIT through `integral-core`'s `Initialization` and
  waits for "finished". `InitArgs` (`<TARGET> --part --partial --key`) and
  `InitKind` are its arguments. A Partial Init reads the Part's Bank Select
  MSB first: `--partial` (from 1) is for SN Synth and PCM Synth tones,
  `--key` (a MIDI key number) for drum kits. `EXIT_INIT_UNCONFIRMED = 2`.
- `part.rs`: `show` and `set` are the `part` subcommands, by the field
  names, ranges and displays of core's `part_settings` registry.
  - `settings_blocks` names the mixer-load blocks that hold a Part's
    settings, which both read first.
  - `parse_field`, `parse_value` (the display form, through core's
    `HostFormat::parse`) and `display` map names and values.
  - `set_in` sets a field in a `DeviceState` through `set_part_setting`
    and returns a `SetOutcome`; `render_show` and `render_set` print.
- `svd.rs`: `svd_validate` decodes one SN-S entry from an SVD file and
  compares it, byte by byte, with the Part tone read from the device.
- `tone_waves.rs`: `read(dev, part)` names the Waves used by a Part's
  Temporary tone (PCM Synth, PCM Drum, SN Synth). `tone_type_name` gives a
  tone type's CLI name (`pcm-synth`). The line builders
  `pcm_synth_lines`, `pcm_drum_lines`, `sn_synth_lines` and `no_waves_line`
  are public.
- `system.rs`: `show` and `set` read and write System Common by the
  field names of core's `system` registry. `parse_set` turns a field name
  and a display value into a `SystemField` and its raw value, and
  `render_show` renders one `name  value` line per field, the values aligned.
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
  - `init`: 1 means nothing was sent (every argument is checked first, and
    a Partial Init's partial or key against the Part's tone type), and 2
    means the Init was sent but "finished" never came.
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
- `part set` refuses an unknown field or a value outside the field's range
  before it connects. It reads the Part before writing, so a key or
  velocity Lower is held at or under its Upper (and an Upper at or over its
  Lower), as `DeviceState::set_part_setting` does; it says so on `stderr`.
  A Scale Tune Type or Key write also sends that call's re-read of the
  Part's settings, whose reply the CLI doesn't wait for.
- `system set` checks the field and value before opening a port, sends one
  DT1 through `DeviceState::set_system_field`, and prints core's
  `VOLATILE_NOTICE`: no SysEx saves System edits.
- DT1 bursts go through `Throttle`, which uses `DeviceState`'s raw queue so
  that writes are not merged.

## Side Effects

- MIDI out:
  - Identity Request.
  - RQ1 reads: `dump`, `raw`, `svd`, `tone_waves`, `expansion`, `part`,
    `pack export`, `system show`, and the `pack import` slot-name reads.
  - DT1 writes: `write`, `raw-send`, `part set`, `expansion load`,
    `pack import`, `pack reinit` and `system set`.
  - The Init request (an RQ1 at `0F 00 18 TT`): `init`. It changes only
    Temporary memory.
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
