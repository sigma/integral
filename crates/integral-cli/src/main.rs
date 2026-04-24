use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;

mod commands;
mod hex;
mod midi;

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
        /// Hex size to read (e.g. 10 for 16 bytes)
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
    },
    /// List the contents of an SVD backup file.
    SvdList {
        /// Path to the .SVD file.
        file: PathBuf,
        /// Filter by chunk type (e.g. "sn-synth", "pcm-drum").
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Filter by category number.
        #[arg(long, short = 'c')]
        category: Option<u8>,
        /// Filter by name pattern (case-insensitive substring match).
        #[arg(long, short = 'n')]
        name: Option<String>,
        /// Output format: text (default), json, markdown.
        #[arg(long, short = 'f', default_value = "text")]
        format: String,
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
    /// Export an SN-S tone from the device to an SVD file.
    SvdExport {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// Output .SVD file path.
        file: PathBuf,
        /// Part number to read from (1-16).
        #[arg(long, default_value_t = 1)]
        part: u8,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
    },
    /// Import patches from an SVD file to the device.
    SvdImport {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        /// Path to the .SVD file.
        file: PathBuf,
        /// Part number to write to (1-16, default 1).
        #[arg(long, default_value_t = 1)]
        part: u8,
        /// Patch index to import (1-based). Omit to import all.
        #[arg(long)]
        index: Option<usize>,
        /// Only show what would be sent (don't actually send).
        #[arg(long)]
        dry_run: bool,
        /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
        #[arg(long, value_parser = parse_device_id)]
        device_id: Option<u8>,
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
            commands::raw::raw_rq1_multi(
                &port,
                Duration::from_secs_f64(timeout),
                did,
                &addr,
                &size,
            )
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
        Cli::Monitor { port } => commands::monitor::monitor(&port),
        Cli::SvdList {
            file,
            r#type,
            category,
            name,
            format,
        } => commands::svd::svd_list(&file, r#type.as_deref(), category, name.as_deref(), &format),
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
        Cli::SvdExport {
            port,
            timeout,
            file,
            part,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::svd::svd_export(&port, Duration::from_secs_f64(timeout), did, &file, part)
        }
        Cli::SvdImport {
            port,
            file,
            part,
            index,
            dry_run,
            device_id,
        } => {
            let did = resolve_device_id(&port, device_id)?;
            commands::svd::svd_import(&port, did, &file, part, index, dry_run)
        }
    }
}
