use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod device;
mod hex;
mod midi;

use commands::expansion::OutputFormat;
use commands::pack::{ListFormat, UserSlots};
use commands::pack_import::ImportOptions;
use integral_core::device_spec::INTEGRA7;
use midi::DEFAULT_PORT_PATTERN;

/// Integral -- CLI tools for the Roland INTEGRA-7.
#[derive(Parser)]
#[command(version)]
enum Cli {
    /// Ping the INTEGRA-7 via SysEx Identity Request.
    Ping {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        #[arg(long)]
        quiet: bool,
    },
    /// Read a parameter from the device via RQ1.
    Read {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// What to read: studio-set-name, master-level, part-mixer, tone-name
        what: String,
        /// Part number (1-16), required for part-specific reads.
        #[arg(long)]
        part: Option<u8>,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// Write a parameter to the device via DT1.
    Write {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        /// What to write: part-level, part-pan, part-mute
        what: String,
        /// Part number (1-16).
        #[arg(long)]
        part: u8,
        /// Value to set (0-127 for level/pan, 0-1 for mute).
        #[arg(long)]
        value: u8,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// Send RQ1 and capture all responses (for multi-response queries).
    RawRq1 {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        /// 4-byte hex address
        addr: String,
        /// 4-byte hex size
        size: String,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// Read raw bytes from a SysEx address (hex).
    RawRead {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        /// 4-byte hex address (e.g. 10000000)
        addr: String,
        /// Size to read: a decimal byte count (e.g. 145), or the 7-bit
        /// SysEx size field as 8 hex digits (e.g. 00000111 = 145 bytes)
        size: String,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// Send raw hex bytes as SysEx and capture all responses.
    RawHex {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        /// Raw hex string (e.g. "F04110000064110F000302550017F7")
        hex: String,
    },
    /// Send a raw DT1 message and capture responses.
    RawSend {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 3.0)]
        timeout: f64,
        /// 4-byte hex address
        addr: String,
        /// Hex data bytes (e.g. "5500" for two bytes 0x55 0x00)
        data: String,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// Dump all incoming MIDI messages (for debugging).
    Monitor {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        /// Also forward traffic both ways between `--port` and this port,
        /// logging each direction (to sniff another app talking to the device).
        #[arg(long)]
        bridge: Option<String>,
    },
    /// Validate SVD decode against the device by comparing SysEx reads.
    SvdValidate {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// Path to the .SVD file.
        file: PathBuf,
        /// Part number to read from (1-16, must have the right tone loaded).
        #[arg(long, default_value_t = 1)]
        part: u8,
        /// 1-based index of the SVD entry to validate.
        #[arg(long)]
        index: usize,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// List, export and import Packs: Studio Sets and tones in one .SVD file.
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// List the Expansions and load them into the Expansion slots A-D.
    Expansion {
        #[command(subcommand)]
        command: ExpansionCommand,
    },
}

/// `expansion` subcommands.
#[derive(Subcommand)]
enum ExpansionCommand {
    /// Show the Expansion slots, then every Expansion and the slot holding it.
    List {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Load Expansions into slots. All sound stops while loading.
    ///
    /// Exit status: 0 success or nothing to do; 1 nothing sent (bad argument,
    /// no device, confirmation refused); 2 load sent but failed or timed out.
    Load {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
        /// Slot assignments such as `A=SRX-01 C=ExSN3 D=OFF` (unnamed slots
        /// keep their Expansion), or `ExPCM` alone to fill all four.
        #[arg(required = true)]
        assignments: Vec<String>,
        /// Print the plan and stop.
        #[arg(long)]
        dry_run: bool,
        /// Load even if Parts would go silent, without asking.
        #[arg(long, short = 'y')]
        yes: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

/// `pack` subcommands.
#[derive(Subcommand)]
enum PackCommand {
    /// List a Pack file's entries as `<type>:<n>` ids (sound export or backup).
    List {
        /// Path to the .SVD file.
        file: PathBuf,
        /// Only entries of this type.
        #[arg(long, short = 't', value_parser = commands::pack::type_parser())]
        r#type: Option<String>,
        /// Only entries with this category number.
        #[arg(long, short = 'c')]
        category: Option<u8>,
        /// Only names containing this (case-insensitive).
        #[arg(long, short = 'n')]
        name: Option<String>,
        #[arg(long, short = 'f', value_enum, default_value_t = ListFormat::Text)]
        format: ListFormat,
    },
    /// Export User slots to a Pack file. Only reads from the device; the
    /// User tones of the chosen Studio Sets are added automatically.
    Export {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
        /// Output .SVD file path.
        file: PathBuf,
        #[command(flatten)]
        slots: UserSlots,
        /// Also export the tone on Part N, unsaved edits included.
        #[arg(long, value_name = "N",
              value_parser = clap::value_parser!(u8).range(1..=i64::from(INTEGRA7.part_count)))]
        current_part: Option<u8>,
        /// Also export the current Studio Set, unsaved edits included.
        #[arg(long)]
        current_set: bool,
        /// Overwrite the output file if it exists.
        #[arg(long)]
        force: bool,
    },
    /// Import a Pack file into User memory. Prints the plan first; a lone
    /// tone or Studio Set loads into Temporary instead.
    ///
    /// Exit status: 0 everything stored; 1 error before anything was
    /// stored; 2 partial store.
    Import {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
        /// Path to the .SVD file (sound export or backup).
        file: PathBuf,
        #[command(flatten)]
        options: ImportOptions,
    },
}

/// Parse a hex device ID string (e.g. "10" -> 0x10).
fn parse_device_id(s: &str) -> Result<u8, String> {
    u8::from_str_radix(s, 16).map_err(|e| format!("invalid hex device ID: {e}"))
}

/// Resolve the device ID: use the explicit value if provided, otherwise
/// auto-detect via identity request, falling back to the default.
fn resolve_device_id(port_pattern: &str, explicit: Option<u8>) -> Result<u8> {
    if let Some(id) = explicit {
        return Ok(id);
    }
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;
    midi::detect_device_id(&mut conn_out, &rx, Duration::from_secs(2))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Ping {
            port,
            timeout,
            quiet,
        } => {
            commands::ping::ping(&port, Duration::from_secs_f64(timeout), quiet)?;
            Ok(())
        }
        Cli::Read {
            port,
            timeout,
            what,
            part,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::dump::read_param(&port, Duration::from_secs_f64(timeout), did, &what, part)
        }
        Cli::Write {
            port,
            what,
            part,
            value,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::dump::write_param(&port, did, &what, part, value)
        }
        Cli::RawRead {
            port,
            timeout,
            addr,
            size,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::raw::raw_read(&port, Duration::from_secs_f64(timeout), did, &addr, &size)
        }
        Cli::RawHex { port, timeout, hex } => {
            commands::raw::send_raw_hex(&port, Duration::from_secs_f64(timeout), &hex)
        }
        Cli::RawRq1 {
            port,
            timeout,
            addr,
            size,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::raw::raw_rq1_multi(&port, Duration::from_secs_f64(timeout), did, &addr, &size)
        }
        Cli::RawSend {
            port,
            timeout,
            addr,
            data,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::raw::raw_send(&port, Duration::from_secs_f64(timeout), did, &addr, &data)
        }
        Cli::Monitor { port, bridge } => commands::monitor::monitor(&port, bridge.as_deref()),
        Cli::SvdValidate {
            port,
            timeout,
            file,
            part,
            index,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::svd::svd_validate(
                &port,
                Duration::from_secs_f64(timeout),
                did,
                &file,
                part,
                index,
            )
        }
        Cli::Pack {
            command:
                PackCommand::List {
                    file,
                    r#type,
                    category,
                    name,
                    format,
                },
        } => commands::pack::list(&file, r#type.as_deref(), category, name.as_deref(), format),
        Cli::Pack {
            command:
                PackCommand::Export {
                    port,
                    timeout,
                    device_id,
                    file,
                    slots,
                    current_part,
                    current_set,
                    force,
                },
        } => commands::pack::export(
            &port,
            device_id,
            Duration::from_secs_f64(timeout),
            &file,
            slots,
            current_part,
            current_set,
            force,
        ),
        Cli::Pack {
            command:
                PackCommand::Import {
                    port,
                    timeout,
                    device_id,
                    file,
                    options,
                },
        } => {
            let code = commands::pack_import::import(
                &port,
                device_id,
                Duration::from_secs_f64(timeout),
                &file,
                &options,
            )?;
            if code != 0 {
                std::process::exit(code.into());
            }
            Ok(())
        }
        Cli::Expansion {
            command:
                ExpansionCommand::List {
                    port,
                    timeout,
                    device_id,
                    format,
                },
        } => commands::expansion::list(&port, device_id, Duration::from_secs_f64(timeout), format),
        Cli::Expansion {
            command:
                ExpansionCommand::Load {
                    port,
                    timeout,
                    device_id,
                    assignments,
                    dry_run,
                    yes,
                    format,
                },
        } => {
            let code = commands::expansion::load(
                &port,
                device_id,
                Duration::from_secs_f64(timeout),
                &assignments,
                dry_run,
                yes,
                format,
            )?;
            if code != 0 {
                std::process::exit(code.into());
            }
            Ok(())
        }
    }
}
