use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};

mod commands;
mod device;
mod hex;
mod midi;
#[cfg(unix)]
mod service;

use commands::expansion::OutputFormat;
use commands::pack::{ListFormat, UserSlots};
use commands::pack_import::{ImportOptions, ReinitOptions};
use integral_core::device_spec::INTEGRA7;
use midi::{Connection, DEFAULT_PORT_PATTERN, Endpoint};

/// Integral -- CLI tools for the Roland INTEGRA-7.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Reach the device through a running Device service (`integral bridge`)
    /// instead of opening a MIDI port. Optionally names its Unix socket
    /// (`--service=PATH`); defaults to the one the plugins look for.
    /// `--port` is then ignored; `monitor` and `bridge` refuse it.
    #[arg(
        long,
        global = true,
        num_args = 0..=1,
        require_equals = true,
        value_name = "PATH"
    )]
    service: Option<Option<PathBuf>>,
    #[command(subcommand)]
    command: Command,
}

/// The `integral` subcommands.
#[derive(Subcommand)]
enum Command {
    /// Ping the INTEGRA-7 via SysEx Identity Request.
    Ping {
        #[command(flatten)]
        conn: PortTimeoutArgs,
        #[arg(long)]
        quiet: bool,
    },
    /// Read a parameter from the device via RQ1.
    Read {
        #[command(flatten)]
        conn: PortTimeoutArgs,
        /// What to read: studio-set-name, master-level, part-mixer, tone-name,
        /// tone-waves
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
        #[command(flatten)]
        conn: RawConnectionArgs,
        /// 4-byte hex address
        addr: String,
        /// 4-byte hex size
        size: String,
    },
    /// Read raw bytes from a SysEx address (hex).
    RawRead {
        #[command(flatten)]
        conn: RawConnectionArgs,
        /// 4-byte hex address (e.g. 10000000)
        addr: String,
        /// Size to read: a decimal byte count (e.g. 145), or the 7-bit
        /// SysEx size field as 8 hex digits (e.g. 00000111 = 145 bytes)
        size: String,
    },
    /// Send raw hex bytes as SysEx and capture all responses.
    RawHex {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        /// Seconds to wait for replies [default: 5, 8 with --service].
        #[arg(long)]
        timeout: Option<f64>,
        /// Raw hex string (e.g. "F04110000064110F000302550017F7")
        hex: String,
    },
    /// Send a raw DT1 message and capture responses.
    RawSend {
        #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
        port: String,
        /// Seconds to wait for replies [default: 3, 6 with --service].
        #[arg(long)]
        timeout: Option<f64>,
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
    /// Run the Device service: it owns the link to the INTEGRA-7 (Wireless,
    /// RTP-MIDI or USB) and exposes the MIDI port "Integral Device Service"
    /// for notes (runs until Ctrl+C).
    #[cfg(unix)]
    Bridge(commands::bridge::BridgeArgs),
    /// Validate SVD decode against the device by comparing SysEx reads.
    SvdValidate {
        #[command(flatten)]
        conn: PortTimeoutArgs,
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
    /// List, export and import Packs: Studio Sets and tones in one .SVD file
    /// (also read from .syx and .mid SysEx dumps).
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// List the Expansions and load them into the Expansion slots A-D.
    Expansion {
        #[command(subcommand)]
        command: ExpansionCommand,
    },
    /// Browse the Wave catalog (offline).
    Wave {
        #[command(subcommand)]
        command: WaveCommand,
    },
}

/// The `--port` and `--timeout` options shared by the subcommands that
/// talk to the device.
#[derive(Args)]
struct PortTimeoutArgs {
    #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
    port: String,
    /// Seconds to wait for a reply [default: 2, 5 with --service].
    #[arg(long)]
    timeout: Option<f64>,
}

impl PortTimeoutArgs {
    /// The `--timeout` value, or the default (longer through the service).
    fn timeout(&self, service: bool) -> Duration {
        midi::timeout(self.timeout, 2.0, service)
    }
}

/// The `--port`, `--timeout` and `--device-id` options shared by the
/// subcommands that address the device.
#[derive(Args)]
struct ConnectionArgs {
    #[command(flatten)]
    base: PortTimeoutArgs,
    /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
    #[arg(long, value_parser = parse_device_id)]
    device_id: Option<u8>,
}

impl ConnectionArgs {
    /// The `--port` value.
    fn port(&self) -> &str {
        &self.base.port
    }

    /// The `--timeout` value, or the default (longer through the service).
    fn timeout(&self, service: bool) -> Duration {
        self.base.timeout(service)
    }
}

/// The connection options of the raw read commands, which default to a
/// longer `--timeout`.
#[derive(Args)]
struct RawConnectionArgs {
    #[arg(long, default_value = DEFAULT_PORT_PATTERN)]
    port: String,
    /// Seconds to wait for a reply [default: 5, 8 with --service].
    #[arg(long)]
    timeout: Option<f64>,
    /// SysEx device ID (hex, e.g. 10). Auto-detected if omitted.
    #[arg(long, value_parser = parse_device_id)]
    device_id: Option<u8>,
}

impl RawConnectionArgs {
    /// The `--timeout` value, or the default (longer through the service).
    fn timeout(&self, service: bool) -> Duration {
        midi::timeout(self.timeout, 5.0, service)
    }
}

/// `wave` subcommands.
#[derive(Subcommand)]
enum WaveCommand {
    /// List the Wave Groups, one group's Waves, or every Wave.
    ///
    /// Exit status: 0 success; 1 unknown Wave Group.
    List {
        /// Wave Group id: INT, SRX-01..SRX-12 or SN-Synth (case-insensitive).
        group: Option<String>,
        /// Print `<group> <number> <name>` for every Wave of every group.
        #[arg(long, conflicts_with = "group")]
        all: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

/// `expansion` subcommands.
#[derive(Subcommand)]
enum ExpansionCommand {
    /// Show the Expansion slots, then every Expansion and the slot holding it.
    List {
        #[command(flatten)]
        conn: ConnectionArgs,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Load Expansions into slots. All sound stops while loading.
    ///
    /// Exit status: 0 success or nothing to do; 1 nothing sent (bad argument,
    /// no device, confirmation refused); 2 load sent but failed or timed out.
    Load {
        #[command(flatten)]
        conn: ConnectionArgs,
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
        /// Path to the .SVD, .syx or .mid file.
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
        #[command(flatten)]
        conn: ConnectionArgs,
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
    /// Check how Pack files convert, offline: read every file under the
    /// paths and total the Conversion reports of their Foreign patches.
    ///
    /// Files that are not Packs are skipped and counted. Exit status: 0
    /// checked; 1 a path can't be read.
    Check {
        /// Pack files, or directories to search recursively.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// How many unmapped Waves and dropped or substituted effects the totals
        /// list.
        #[arg(long, default_value_t = 10)]
        top: usize,
        #[arg(long, short = 'f', value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Import a Pack file into User memory. Prints the plan first; a lone
    /// tone or Studio Set loads into Temporary instead.
    ///
    /// Exit status: 0 everything stored; 1 error before anything was
    /// stored; 2 partial store.
    Import {
        #[command(flatten)]
        conn: ConnectionArgs,
        /// Path to the .SVD file (sound export or backup), or a .syx or .mid
        /// SysEx dump.
        file: PathBuf,
        #[command(flatten)]
        options: ImportOptions,
    },
    /// Store INIT into User slots, e.g. `pcm-synth 113` or `sn-synth 1-4`.
    /// Prints the slots' names and asks first; exports the used ones to an
    /// undo file.
    ///
    /// Exit status: 0 every slot stored; 1 error before anything was
    /// stored; 2 partial store.
    Reinit {
        #[command(flatten)]
        conn: ConnectionArgs,
        #[command(flatten)]
        options: ReinitOptions,
    },
}

/// Parse a hex device ID string (e.g. "10" -> 0x10).
fn parse_device_id(s: &str) -> Result<u8, String> {
    u8::from_str_radix(s, 16).map_err(|e| format!("invalid hex device ID: {e}"))
}

/// Resolve the device ID: use the explicit value if provided, otherwise
/// auto-detect via identity request, falling back to the default.
fn resolve_device_id(endpoint: Endpoint<'_>, explicit: Option<u8>) -> Result<u8> {
    if let Some(id) = explicit {
        return Ok(id);
    }
    let Connection {
        output: mut conn_out,
        rx,
    } = midi::open_midi(endpoint)?;
    midi::detect_device_id(
        &mut conn_out,
        &rx,
        midi::timeout(None, 2.0, matches!(endpoint, Endpoint::Service(_))),
    )
}

/// Exit the process with `code` unless it is zero.
fn exit_nonzero(code: u8) -> Result<()> {
    if code != 0 {
        std::process::exit(code.into());
    }
    Ok(())
}

impl Command {
    /// Why this command cannot run with `--service`, if it cannot.
    fn refuses_service(&self) -> Option<&'static str> {
        match self {
            Self::Monitor { .. } => {
                Some("monitor dumps all MIDI, but the service shares only SysEx")
            }
            Self::Bridge(_) => Some("bridge runs the Device service itself"),
            Self::Expansion {
                command: ExpansionCommand::Load { .. },
            } => Some(
                "expansion load needs the service to hold its queue for a load it starts itself",
            ),
            _ => None,
        }
    }
}

fn main() -> Result<()> {
    let Cli { service, command } = Cli::parse();
    if service.is_some()
        && let Some(why) = command.refuses_service()
    {
        bail!("--service does not apply: {why}");
    }
    // Where a command reaches the device: the service, else its `--port`.
    let service = service.as_ref().map(|socket| socket.as_deref());
    let slow = service.is_some();

    match command {
        Command::Ping { conn, quiet } => {
            commands::ping::ping(
                Endpoint::select(service, &conn.port),
                conn.timeout(slow),
                quiet,
            )?;
            Ok(())
        }
        Command::Read {
            conn,
            what,
            part,
            device_id,
        } => {
            let did = resolve_device_id(Endpoint::select(service, &conn.port), device_id)?;
            commands::dump::read_param(
                Endpoint::select(service, &conn.port),
                conn.timeout(slow),
                did,
                &what,
                part,
            )
        }
        Command::Write {
            port,
            what,
            part,
            value,
            device_id,
        } => {
            let did = resolve_device_id(Endpoint::select(service, &port), device_id)?;
            commands::dump::write_param(Endpoint::select(service, &port), did, &what, part, value)
        }
        Command::RawRead { conn, addr, size } => {
            let did = resolve_device_id(Endpoint::select(service, &conn.port), conn.device_id)?;
            commands::raw::raw_read(
                Endpoint::select(service, &conn.port),
                conn.timeout(slow),
                did,
                &addr,
                &size,
            )
        }
        Command::RawHex { port, timeout, hex } => commands::raw::send_raw_hex(
            Endpoint::select(service, &port),
            midi::timeout(timeout, 5.0, slow),
            &hex,
        ),
        Command::RawRq1 { conn, addr, size } => {
            let did = resolve_device_id(Endpoint::select(service, &conn.port), conn.device_id)?;
            commands::raw::raw_rq1_multi(
                Endpoint::select(service, &conn.port),
                conn.timeout(slow),
                did,
                &addr,
                &size,
            )
        }
        Command::RawSend {
            port,
            timeout,
            addr,
            data,
            device_id,
        } => {
            let did = resolve_device_id(Endpoint::select(service, &port), device_id)?;
            commands::raw::raw_send(
                Endpoint::select(service, &port),
                midi::timeout(timeout, 3.0, slow),
                did,
                &addr,
                &data,
            )
        }
        Command::Monitor { port, bridge } => commands::monitor::monitor(&port, bridge.as_deref()),
        #[cfg(unix)]
        Command::Bridge(args) => commands::bridge::bridge(&args),
        Command::SvdValidate {
            conn,
            file,
            part,
            index,
            device_id,
        } => {
            let did = resolve_device_id(Endpoint::select(service, &conn.port), device_id)?;
            commands::svd::svd_validate(
                Endpoint::select(service, &conn.port),
                conn.timeout(slow),
                did,
                &file,
                part,
                index,
            )
        }
        Command::Pack {
            command:
                PackCommand::List {
                    file,
                    r#type,
                    category,
                    name,
                    format,
                },
        } => commands::pack::list(&file, r#type.as_deref(), category, name.as_deref(), format),
        Command::Pack {
            command:
                PackCommand::Export {
                    conn,
                    file,
                    slots,
                    current_part,
                    current_set,
                    force,
                },
        } => commands::pack::export(
            Endpoint::select(service, conn.port()),
            conn.device_id,
            conn.timeout(slow),
            &file,
            slots,
            current_part,
            current_set,
            force,
        ),
        Command::Pack {
            command: PackCommand::Check { paths, top, format },
        } => exit_nonzero(commands::pack_check::run(&paths, top, format)?),
        Command::Pack {
            command:
                PackCommand::Import {
                    conn,
                    file,
                    options,
                },
        } => exit_nonzero(commands::pack_import::import(
            Endpoint::select(service, conn.port()),
            conn.device_id,
            conn.timeout(slow),
            &file,
            &options,
        )?),
        Command::Pack {
            command: PackCommand::Reinit { conn, options },
        } => exit_nonzero(commands::pack_import::reinit(
            Endpoint::select(service, conn.port()),
            conn.device_id,
            conn.timeout(slow),
            &options,
        )?),
        Command::Expansion {
            command: ExpansionCommand::List { conn, format },
        } => commands::expansion::list(
            Endpoint::select(service, conn.port()),
            conn.device_id,
            conn.timeout(slow),
            format,
        ),
        Command::Expansion {
            command:
                ExpansionCommand::Load {
                    conn,
                    assignments,
                    dry_run,
                    yes,
                    format,
                },
        } => exit_nonzero(commands::expansion::load(
            Endpoint::select(service, conn.port()),
            conn.device_id,
            conn.timeout(slow),
            &assignments,
            dry_run,
            yes,
            format,
        )?),
        Command::Wave {
            command: WaveCommand::List { group, all, format },
        } => commands::wave::list(group.as_deref(), all, format),
    }
}
