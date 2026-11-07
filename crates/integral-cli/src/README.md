# integral-cli/src

## Overview

The `integral` binary, a clap CLI for the INTEGRA-7. It talks SysEx (Identity
Request, RQ1 reads, DT1 writes) over `midir` ports chosen by a
case-insensitive name pattern (default `Integra`). `main.rs` only defines the
commands and dispatches them. The shared connection, request and hex helpers
live in sibling modules, and the command bodies live in
[`./commands/`](./commands/README.md).

## Interface

- `main.rs`
  - `Cli` defines the subcommands:
    - device commands: `ping`, `read`, `write`, `monitor`, `svd-validate`
    - raw SysEx: `raw-rq1`, `raw-read`, `raw-hex`, `raw-send`
    - `bridge` (Unix only)
    - `pack {list, export, check, import, reinit}`
    - `expansion {list, load}`
    - `wave {list}`
  - The shared option groups are `PortTimeoutArgs`, `ConnectionArgs` and
    `RawConnectionArgs` (`--port`, `--timeout`, `--device-id`).
  - `resolve_device_id` returns the explicit device ID, or detects it.
  - Helpers: `parse_device_id` and `exit_nonzero`.
- `device.rs`: `IntegraDevice` is the device handle.
  - `connect` and `connect_detecting` open it.
  - `request_data` (RQ1) and `send_data` (DT1) read and write.
  - Typed reads: `read_studio_set_name`, `read_master_level`,
    `read_surround_enabled` and `read_part_mixer`.
  - `quiet` turns off per-request RQ1/DT1 logging. `conn_out_mut`, `rx`, `device_id` and
    `timeout` give low-level access.
- `midi.rs`
  - `DEFAULT_PORT_PATTERN` is `Integra`, and `DEFAULT_DEVICE_ID` is core's
    `sysex::DEFAULT_DEVICE_ID`, the fallback when detection fails.
  - `open_midi` returns a `MidiConnection`. the port is a name-matched input and output pair.
  - `read_block` and `request_data` read a block, `detect_device_id` finds
    the device ID, and `format_ranges` prints byte ranges.
- `hex.rs`: `parse_hex_addr`, `parse_hex_size`, `parse_read_size`,
  `parse_hex_bytes` and `format_hex`.
- [`./commands/README.md`](./commands/README.md): the subcommand modules.

## Invariants

- Each address byte must be in `00-7F`.
- `parse_read_size` treats exactly 8 hex digits as a 7-bit SysEx size, and
  anything else as a decimal byte count.
- `parse_hex_bytes` needs an even number of hex characters.
- Port choice: when several ports match, the first one that answers an
  Identity Request within 500 ms is used. If none answers, the first match
  is used.
- The device ID is detected from the Identity Reply. `0x10` is used only as
  the fallback.
- `request_data` fails unless every byte of the block arrives.
- `IntegraDevice` must hold the input connection guard, or the receiver gets
  nothing.
- Parts are numbered 1-16 on the command line and 0-15 inside
  `IntegraDevice`.

## Side Effects

- Opens the `midir` clients `integral-out` and `integral-in`. The input
  callback runs on `midir`'s thread and forwards messages over an
  `mpsc::channel`.
- Sends SysEx to the hardware. `write`, `raw-send`, `pack import`,
  `pack reinit` and `expansion load` change device state, and an Expansion
  load stops all sound while it runs.
- Logs to `stderr`: port choice, the detected device ID and the fallback
  warning always; RQ1/DT1 traffic unless `quiet()` is set.

## Verification

- Run: `cargo test -p integral-cli`
