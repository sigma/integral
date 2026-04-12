use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use integral_core::address::{Address, DataSize};
use integral_core::mfx;
use integral_core::params;
use integral_core::sn_synth;
use integral_core::svd::{ChunkType, SvdChunk, SvdFile, tone_category_name};
use integral_core::svd_convert::sysex_to_svd;
use integral_core::svd_convert::{sns_to_dt1s, svd_to_sysex};
use integral_core::svd_specs::{SNA_TONE_SPEC, SNS_TONE_SPEC};
use integral_core::sysex;
use midir::{MidiInput, MidiOutput};

/// Integral — CLI tools for the Roland INTEGRA-7.
#[derive(Parser)]
#[command(version)]
enum Cli {
    /// Ping the INTEGRA-7 via SysEx Identity Request.
    Ping {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        #[arg(long)]
        quiet: bool,
    },
    /// Read a parameter from the device via RQ1.
    Read {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// What to read: studio-set-name, master-level, part-mixer, tone-name
        what: String,
        /// Part number (1-16), required for part-specific reads.
        #[arg(long)]
        part: Option<u8>,
    },
    /// Write a parameter to the device via DT1.
    Write {
        #[arg(long, default_value = "Integra")]
        port: String,
        /// What to write: part-level, part-pan, part-mute
        what: String,
        /// Part number (1-16).
        #[arg(long)]
        part: u8,
        /// Value to set (0-127 for level/pan, 0-1 for mute).
        #[arg(long)]
        value: u8,
    },
    /// Send RQ1 and capture all responses (for multi-response queries). E.g.: raw-rq1 0F000302 00000540
    RawRq1 {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        /// 4-byte hex address
        addr: String,
        /// 4-byte hex size
        size: String,
    },
    /// Read raw bytes from a SysEx address (hex). E.g.: raw-read 10000000 10
    RawRead {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        /// 4-byte hex address (e.g. 10000000)
        addr: String,
        /// Hex size to read (e.g. 10 for 16 bytes)
        size: String,
    },
    /// Send raw hex bytes as SysEx and capture all responses.
    RawHex {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 5.0)]
        timeout: f64,
        /// Raw hex string (e.g. "F04110000064110F000302550017F7")
        hex: String,
    },
    /// Probe undocumented command IDs to find catalog request format.
    Probe {
        #[arg(long, default_value = "Integra")]
        port: String,
    },
    /// Send a raw DT1 message and capture responses. E.g.: raw-send 0F000302 5500
    RawSend {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 3.0)]
        timeout: f64,
        /// 4-byte hex address
        addr: String,
        /// Hex data bytes (e.g. "5500" for two bytes 0x55 0x00)
        data: String,
    },
    /// Dump all incoming MIDI messages (for debugging).
    Monitor {
        #[arg(long, default_value = "Integra")]
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
        #[arg(long, default_value = "Integra")]
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
    },
    /// Export an SN-S tone from the device to an SVD file.
    SvdExport {
        #[arg(long, default_value = "Integra")]
        port: String,
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,
        /// Output .SVD file path.
        file: PathBuf,
        /// Part number to read from (1-16).
        #[arg(long, default_value_t = 1)]
        part: u8,
    },
    /// Import patches from an SVD file to the device.
    SvdImport {
        #[arg(long, default_value = "Integra")]
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
    },
}

fn find_port_by_name<T: midir::MidiIO>(midi_io: &T, pattern: &str) -> Option<T::Port> {
    let pattern_lower = pattern.to_lowercase();
    midi_io.ports().into_iter().find(|port| {
        midi_io
            .port_name(port)
            .is_ok_and(|name| name.to_lowercase().contains(&pattern_lower))
    })
}

/// Open MIDI input+output connections and return (conn_in_guard, conn_out, rx).
fn open_midi(
    port_pattern: &str,
) -> Result<(
    midir::MidiInputConnection<()>,
    midir::MidiOutputConnection,
    mpsc::Receiver<Vec<u8>>,
)> {
    let midi_out = MidiOutput::new("integral-out").context("failed to create MIDI output")?;
    let midi_in = MidiInput::new("integral-in").context("failed to create MIDI input")?;

    let out_port = find_port_by_name(&midi_out, port_pattern)
        .with_context(|| format!("no MIDI output port matching '{port_pattern}'"))?;
    let in_port = find_port_by_name(&midi_in, port_pattern)
        .with_context(|| format!("no MIDI input port matching '{port_pattern}'"))?;

    let port_name = midi_out
        .port_name(&out_port)
        .unwrap_or_else(|_| port_pattern.to_string());
    eprintln!("Using MIDI port: {port_name}");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let conn_in = midi_in
        .connect(
            &in_port,
            "integral-in",
            move |_timestamp, message, _| {
                let _ = tx.send(message.to_vec());
            },
            (),
        )
        .context("failed to connect MIDI input")?;

    let conn_out = midi_out
        .connect(&out_port, "integral-out")
        .context("failed to connect MIDI output")?;

    Ok((conn_in, conn_out, rx))
}

const DEVICE_ID: u8 = 0x10;

/// Send an RQ1 and wait for the DT1 response.
fn request_data(
    conn_out: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    address: &Address,
    size: &DataSize,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let rq1 = sysex::build_rq1(DEVICE_ID, address, size);
    eprintln!("Sending RQ1: addr={}, size={}", address, size);
    eprintln!(
        "  Raw: {}",
        rq1.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );
    conn_out.send(&rq1).context("failed to send RQ1")?;

    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            bail!("RQ1 timeout waiting for response at address {address}");
        }
        match rx.recv_timeout(remaining) {
            Ok(data) => {
                // Print every incoming message for debugging
                eprintln!(
                    "  Received ({} bytes): {}",
                    data.len(),
                    data.iter()
                        .take(20)
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ")
                );

                match sysex::parse_dt1(&data) {
                    Ok(dt1) => {
                        eprintln!(
                            "  Parsed DT1: addr={}, {} data bytes",
                            dt1.address,
                            dt1.data.len()
                        );
                        if dt1.address == *address {
                            return Ok(dt1.data);
                        }
                        eprintln!("  (address mismatch, continuing...)");
                    }
                    Err(e) => {
                        eprintln!("  (not a DT1: {e})");
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                bail!("RQ1 timeout waiting for response at address {address}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("MIDI input disconnected");
            }
        }
    }
}

fn ping(port_pattern: &str, timeout: Duration, quiet: bool) -> Result<()> {
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    if !quiet {
        eprint!("Pinging... ");
    }

    conn_out
        .send(&sysex::identity_request())
        .context("failed to send identity request")?;

    loop {
        match rx.recv_timeout(timeout) {
            Ok(data) => match sysex::parse_identity_reply(&data) {
                Ok(identity) => {
                    if !quiet {
                        if identity.is_integra7() {
                            eprintln!("OK");
                        } else {
                            eprintln!("OK (not an Integra-7)");
                        }
                        println!("  Device ID:     {:02X}H", identity.device_id);
                        println!(
                            "  Manufacturer:  {:02X}H{}",
                            identity.manufacturer_id,
                            if identity.manufacturer_id == sysex::ROLAND_ID {
                                " (Roland)"
                            } else {
                                ""
                            }
                        );
                        println!(
                            "  Family:        {:02X} {:02X}",
                            identity.family_code[0], identity.family_code[1]
                        );
                        println!(
                            "  Family Number: {:02X} {:02X}",
                            identity.family_number[0], identity.family_number[1]
                        );
                        println!(
                            "  Revision:      {:02X} {:02X} {:02X} {:02X}",
                            identity.revision[0],
                            identity.revision[1],
                            identity.revision[2],
                            identity.revision[3]
                        );
                    }
                    return Ok(());
                }
                Err(_) => continue,
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !quiet {
                    eprintln!("FAIL: No response");
                }
                bail!("no response from device");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
        }
    }
}

fn read_param(port_pattern: &str, timeout: Duration, what: &str, part: Option<u8>) -> Result<()> {
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;
    let timeout = Duration::from_secs_f64(timeout.as_secs_f64());

    match what {
        "studio-set-name" => {
            let data = request_data(
                &mut conn_out,
                &rx,
                &params::STUDIO_SET_NAME,
                &params::STUDIO_SET_NAME_SIZE,
                timeout,
            )?;
            let name = String::from_utf8_lossy(&data);
            println!("Studio Set Name: \"{name}\"");
        }
        "master-level" => {
            let data = request_data(
                &mut conn_out,
                &rx,
                &params::SYSTEM_MASTER_LEVEL,
                &params::SINGLE_BYTE_SIZE,
                timeout,
            )?;
            println!("Master Level: {}", data[0]);
        }
        "part-mixer" => {
            let p = part.context("--part is required for part-mixer")? - 1;
            let addr = params::part_address(p, params::part::RECEIVE_CHANNEL);
            let data = request_data(&mut conn_out, &rx, &addr, &params::PART_MIXER_SIZE, timeout)?;
            println!("Part {} mixer state:", p + 1);
            println!("  Receive Channel: {}", data.first().unwrap_or(&0) + 1);
            println!("  Tone Bank MSB:   {}", data.get(0x06).unwrap_or(&0));
            println!("  Tone Bank LSB:   {}", data.get(0x07).unwrap_or(&0));
            println!("  Tone PC:         {}", data.get(0x08).unwrap_or(&0));
            println!("  Level:           {}", data.get(0x09).unwrap_or(&0));
            println!("  Pan:             {}", data.get(0x0A).unwrap_or(&0));
            println!("  Mute:            {}", data.get(0x25).unwrap_or(&0));
            println!("  Chorus Send:     {}", data.get(0x27).unwrap_or(&0));
            println!("  Reverb Send:     {}", data.get(0x28).unwrap_or(&0));
        }
        "tone-name" => {
            let p = part.context("--part is required for tone-name")? - 1;

            // First read the bank MSB to determine tone type
            let bank_addr = params::part_address(p, params::part::TONE_BANK_MSB);
            let bank_data = request_data(
                &mut conn_out,
                &rx,
                &bank_addr,
                &params::SINGLE_BYTE_SIZE,
                timeout,
            )?;
            let msb = bank_data[0];
            eprintln!("Part {} Bank MSB: {}", p + 1, msb);

            let tt = params::tone_type_from_bank_msb(msb)
                .with_context(|| format!("unknown tone type for bank MSB {msb}"))?;
            let name_addr = params::tone_name_address(p, tt);
            let data = request_data(
                &mut conn_out,
                &rx,
                &name_addr,
                &params::TONE_NAME_SIZE,
                timeout,
            )?;
            let name = String::from_utf8_lossy(&data);
            println!("Part {} Tone Name: \"{name}\"", p + 1);
        }
        _ => bail!(
            "unknown read target: {what}. Use: studio-set-name, master-level, part-mixer, tone-name"
        ),
    }

    Ok(())
}

fn write_param(port_pattern: &str, what: &str, part: u8, value: u8) -> Result<()> {
    let midi_out = MidiOutput::new("integral-out").context("failed to create MIDI output")?;
    let out_port = find_port_by_name(&midi_out, port_pattern)
        .with_context(|| format!("no MIDI output port matching '{port_pattern}'"))?;
    let mut conn_out = midi_out
        .connect(&out_port, "integral-out")
        .context("failed to connect MIDI output")?;

    let p = part - 1;
    let (addr, display) = match what {
        "part-level" => (params::part_address(p, params::part::LEVEL), "Level"),
        "part-pan" => (params::part_address(p, params::part::PAN), "Pan"),
        "part-mute" => (params::part_address(p, params::part::MUTE), "Mute"),
        _ => bail!("unknown write target: {what}. Use: part-level, part-pan, part-mute"),
    };

    let dt1 = sysex::build_dt1(DEVICE_ID, &addr, &[value]);
    eprintln!(
        "Sending DT1: Part {} {} = {} (addr={})",
        part, display, value, addr
    );
    conn_out.send(&dt1).context("failed to send DT1")?;
    println!("OK");
    Ok(())
}

fn monitor(port_pattern: &str) -> Result<()> {
    let (_conn_in, _conn_out, rx) = open_midi(port_pattern)?;
    eprintln!("Monitoring incoming MIDI messages (Ctrl+C to stop)...");

    loop {
        match rx.recv() {
            Ok(data) => {
                let hex: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
                print!("[{} bytes] {}", data.len(), hex.join(" "));

                // Try to parse as DT1
                if let Ok(dt1) = sysex::parse_dt1(&data) {
                    print!("  → DT1 addr={} data=[", dt1.address);
                    for (i, b) in dt1.data.iter().enumerate() {
                        if i > 0 {
                            print!(", ");
                        }
                        print!("{:02X}", b);
                    }
                    print!("]");
                }
                println!();
            }
            Err(_) => bail!("MIDI input disconnected"),
        }
    }
}

fn parse_hex_addr(s: &str) -> Result<Address> {
    let bytes = u32::from_str_radix(s, 16).context("invalid hex address")?;
    Ok(Address::new(
        ((bytes >> 24) & 0xFF) as u8,
        ((bytes >> 16) & 0xFF) as u8,
        ((bytes >> 8) & 0xFF) as u8,
        (bytes & 0xFF) as u8,
    ))
}

fn parse_hex_size(s: &str) -> Result<DataSize> {
    let bytes = u32::from_str_radix(s, 16).context("invalid hex size")?;
    Ok(DataSize::new(
        ((bytes >> 24) & 0xFF) as u8,
        ((bytes >> 16) & 0xFF) as u8,
        ((bytes >> 8) & 0xFF) as u8,
        (bytes & 0xFF) as u8,
    ))
}

fn raw_read(port_pattern: &str, timeout: Duration, addr_hex: &str, size_hex: &str) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let size = parse_hex_size(size_hex)?;
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    let data = request_data(&mut conn_out, &rx, &addr, &size, timeout)?;

    // Print as hex
    let hex: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
    println!("Hex:   {}", hex.join(" "));

    // Print as ASCII (printable chars only)
    let ascii: String = data
        .iter()
        .map(|&b| {
            if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    println!("ASCII: {ascii}");

    Ok(())
}

fn parse_hex_bytes(s: &str) -> Result<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .with_context(|| format!("invalid hex at position {i}"))
        })
        .collect()
}

fn raw_rq1_multi(
    port_pattern: &str,
    timeout: Duration,
    addr_hex: &str,
    size_hex: &str,
) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let size = parse_hex_size(size_hex)?;
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    let rq1 = sysex::build_rq1(DEVICE_ID, &addr, &size);
    eprintln!("Sending RQ1: addr={}, size={}", addr, size);
    conn_out.send(&rq1).context("failed to send RQ1")?;

    let deadline = std::time::Instant::now() + timeout;
    let mut count = 0;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(msg) => {
                count += 1;
                if let Ok(dt1) = sysex::parse_dt1(&msg) {
                    let ascii: String = dt1
                        .data
                        .iter()
                        .map(|&b| {
                            if (0x20..=0x7E).contains(&b) {
                                b as char
                            } else {
                                '.'
                            }
                        })
                        .collect();
                    println!(
                        "[{}] addr={} ({} bytes) {}",
                        count,
                        dt1.address,
                        dt1.data.len(),
                        ascii
                    );
                } else {
                    let hex: Vec<String> = msg.iter().map(|b| format!("{:02X}", b)).collect();
                    println!("[{}] raw ({} bytes): {}", count, msg.len(), hex.join(" "));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("disconnected"),
        }
    }
    eprintln!("Received {} messages", count);
    Ok(())
}

fn send_raw_hex(port_pattern: &str, timeout: Duration, hex: &str) -> Result<()> {
    let bytes = parse_hex_bytes(hex)?;
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    let hex_display: Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    eprintln!("Sending raw: {}", hex_display.join(" "));
    conn_out.send(&bytes).context("send failed")?;

    let deadline = std::time::Instant::now() + timeout;
    let mut count = 0;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(msg) => {
                count += 1;
                if let Ok(dt1) = sysex::parse_dt1(&msg) {
                    let ascii: String = dt1
                        .data
                        .iter()
                        .map(|&b| {
                            if (0x20..=0x7E).contains(&b) {
                                b as char
                            } else {
                                '.'
                            }
                        })
                        .collect();
                    println!(
                        "[{}] addr={} ({} bytes) {}",
                        count,
                        dt1.address,
                        dt1.data.len(),
                        ascii
                    );
                } else {
                    let hex_out: Vec<String> = msg.iter().map(|b| format!("{:02X}", b)).collect();
                    println!("[{}] raw: {}", count, hex_out.join(" "));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("disconnected"),
        }
    }
    eprintln!("Received {} messages", count);
    Ok(())
}

fn probe_catalog(port_pattern: &str) -> Result<()> {
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    let addr: [u8; 4] = [0x0F, 0x00, 0x03, 0x02];
    let data: [u8; 2] = [0x55, 0x00];

    // Try different command IDs
    for cmd_id in [0x0Bu8, 0x0C, 0x0D, 0x0E, 0x0F, 0x11, 0x12, 0x13, 0x14, 0x15] {
        // Build raw SysEx: F0 41 10 00 00 64 CMD addr data chk F7
        let mut chk_data = Vec::new();
        chk_data.extend_from_slice(&addr);
        chk_data.extend_from_slice(&data);
        let chk = sysex::checksum(&chk_data);

        let mut msg = vec![0xF0, 0x41, DEVICE_ID, 0x00, 0x00, 0x64, cmd_id];
        msg.extend_from_slice(&addr);
        msg.extend_from_slice(&data);
        msg.push(chk);
        msg.push(0xF7);

        eprint!("cmd={:#04X}: ", cmd_id);
        conn_out.send(&msg).context("send failed")?;

        std::thread::sleep(Duration::from_millis(200));

        let mut count = 0;
        while let Ok(response) = rx.try_recv() {
            count += 1;
            if count <= 3
                && let Ok(dt1) = sysex::parse_dt1(&response)
            {
                let ascii: String = dt1
                    .data
                    .iter()
                    .map(|&b| {
                        if (0x20..=0x7E).contains(&b) {
                            b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();
                eprint!("[{}] ", ascii);
            }
        }
        eprintln!("{count} response(s)");
    }
    Ok(())
}

fn raw_send(port_pattern: &str, timeout: Duration, addr_hex: &str, data_hex: &str) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let data = parse_hex_bytes(data_hex)?;
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    let dt1 = sysex::build_dt1(DEVICE_ID, &addr, &data);
    eprintln!("Sending DT1: addr={}, data={}", addr, data_hex);
    conn_out.send(&dt1).context("failed to send DT1")?;

    // Capture all responses until timeout
    let deadline = std::time::Instant::now() + timeout;
    let mut count = 0;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match rx.recv_timeout(remaining) {
            Ok(msg) => {
                count += 1;
                if let Ok(dt1) = sysex::parse_dt1(&msg) {
                    let ascii: String = dt1
                        .data
                        .iter()
                        .map(|&b| {
                            if (0x20..=0x7E).contains(&b) {
                                b as char
                            } else {
                                '.'
                            }
                        })
                        .collect();
                    println!(
                        "[{}] addr={} ({} bytes) {}",
                        count,
                        dt1.address,
                        dt1.data.len(),
                        ascii
                    );
                } else {
                    let hex: Vec<String> = msg.iter().map(|b| format!("{:02X}", b)).collect();
                    println!("[{}] raw ({} bytes): {}", count, msg.len(), hex.join(" "));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("disconnected"),
        }
    }
    eprintln!("Received {} messages", count);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli {
        Cli::Ping {
            port,
            timeout,
            quiet,
        } => ping(&port, Duration::from_secs_f64(timeout), quiet),
        Cli::Read {
            port,
            timeout,
            what,
            part,
        } => read_param(&port, Duration::from_secs_f64(timeout), &what, part),
        Cli::Write {
            port,
            what,
            part,
            value,
        } => write_param(&port, &what, part, value),
        Cli::RawRead {
            port,
            timeout,
            addr,
            size,
        } => raw_read(&port, Duration::from_secs_f64(timeout), &addr, &size),
        Cli::RawHex { port, timeout, hex } => {
            send_raw_hex(&port, Duration::from_secs_f64(timeout), &hex)
        }
        Cli::Probe { port } => probe_catalog(&port),
        Cli::RawRq1 {
            port,
            timeout,
            addr,
            size,
        } => raw_rq1_multi(&port, Duration::from_secs_f64(timeout), &addr, &size),
        Cli::RawSend {
            port,
            timeout,
            addr,
            data,
        } => raw_send(&port, Duration::from_secs_f64(timeout), &addr, &data),
        Cli::Monitor { port } => monitor(&port),
        Cli::SvdList {
            file,
            r#type,
            category,
            name,
            format,
        } => svd_list(&file, r#type.as_deref(), category, name.as_deref(), &format),
        Cli::SvdValidate {
            port,
            timeout,
            file,
            part,
            index,
        } => svd_validate(&port, Duration::from_secs_f64(timeout), &file, part, index),
        Cli::SvdExport {
            port,
            timeout,
            file,
            part,
        } => svd_export(&port, Duration::from_secs_f64(timeout), &file, part),
        Cli::SvdImport {
            port,
            file,
            part,
            index,
            dry_run,
        } => svd_import(&port, &file, part, index, dry_run),
    }
}

#[derive(serde::Serialize)]
struct SvdEntry {
    #[serde(rename = "type")]
    chunk_type: String,
    index: usize,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category_name: Option<String>,
}

impl SvdEntry {
    /// Format the category for display: name if known, raw number otherwise, "—" if absent.
    fn category_display(&self) -> String {
        match (self.category, &self.category_name) {
            (_, Some(name)) => name.clone(),
            (Some(id), None) => format!("?{id}"),
            (None, None) => "—".to_string(),
        }
    }
}

/// Extract the patch name from an SVD entry's bitstream.
fn extract_name(entry: &[u8], name_len: usize) -> String {
    let mut reader = integral_core::bitstream::BitReader::new(entry);
    let mut name = String::with_capacity(name_len);
    for _ in 0..name_len {
        if let Ok(ch) = reader.read_bits(7) {
            let ch = ch as u8;
            name.push(if (32..=127).contains(&ch) {
                ch as char
            } else {
                ' '
            });
        }
    }
    name.trim_end().to_string()
}

/// Extract the tone category from an SVD entry, if a spec table is available.
fn extract_category(entry: &[u8], chunk_type: ChunkType) -> Option<u8> {
    match chunk_type {
        ChunkType::SnSynthTone => {
            let sections = svd_to_sysex(entry, &SNS_TONE_SPEC).ok()?;
            Some(sections[0][0x36])
        }
        ChunkType::SnAcousticTone => {
            let sections = svd_to_sysex(entry, &SNA_TONE_SPEC).ok()?;
            Some(sections[0][0x1B])
        }
        _ => None,
    }
}

fn svd_list(
    path: &std::path::Path,
    type_filter: Option<&str>,
    category_filter: Option<u8>,
    name_filter: Option<&str>,
    format: &str,
) -> Result<()> {
    let data = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let svd =
        SvdFile::parse(&data).with_context(|| format!("failed to parse {}", path.display()))?;

    // Collect entries with metadata.
    let mut entries: Vec<SvdEntry> = Vec::new();

    for chunk in &svd.chunks {
        if let Some(tf) = type_filter
            && chunk.chunk_type.cli_name() != tf
        {
            continue;
        }

        let name_len = match chunk.chunk_type {
            ChunkType::StudioSet => 16,
            _ => 12,
        };

        for (i, raw) in chunk.entries.iter().enumerate() {
            let name = extract_name(raw, name_len);
            let category = extract_category(raw, chunk.chunk_type);

            if let Some(cf) = category_filter
                && category != Some(cf)
            {
                continue;
            }
            if let Some(nf) = name_filter
                && !name.to_lowercase().contains(&nf.to_lowercase())
            {
                continue;
            }

            let category_name = category.and_then(tone_category_name).map(str::to_string);
            entries.push(SvdEntry {
                chunk_type: chunk.chunk_type.cli_name().to_string(),
                index: i + 1,
                name,
                category,
                category_name,
            });
        }
    }

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
        "markdown" => {
            print_markdown(&svd, &entries);
        }
        _ => {
            print_text(path, &svd, &entries, type_filter);
        }
    }

    Ok(())
}

fn print_text(
    path: &std::path::Path,
    svd: &SvdFile,
    entries: &[SvdEntry],
    type_filter: Option<&str>,
) {
    println!("SVD file: {}", path.display());
    println!("Chunks: {}", svd.chunks.len());
    println!();

    for chunk in &svd.chunks {
        if let Some(tf) = type_filter
            && chunk.chunk_type.cli_name() != tf
        {
            continue;
        }

        let chunk_entries: Vec<&SvdEntry> = entries
            .iter()
            .filter(|e| e.chunk_type == chunk.chunk_type.cli_name())
            .collect();

        println!(
            "{} ({}): {} entries ({} bytes/entry)",
            chunk.chunk_type,
            std::str::from_utf8(&chunk.chunk_type.to_code()).unwrap_or("????"),
            chunk.entries.len(),
            chunk.entry_size,
        );

        for entry in &chunk_entries {
            if entry.category.is_some() {
                println!(
                    "  {:>3}: {}  [{}]",
                    entry.index,
                    entry.name,
                    entry.category_display()
                );
            } else {
                println!("  {:>3}: {}", entry.index, entry.name);
            }
        }
        println!();
    }
}

fn print_markdown(svd: &SvdFile, entries: &[SvdEntry]) {
    for chunk in &svd.chunks {
        let chunk_entries: Vec<&SvdEntry> = entries
            .iter()
            .filter(|e| e.chunk_type == chunk.chunk_type.cli_name())
            .collect();

        if chunk_entries.is_empty() {
            continue;
        }

        println!("## {}\n", chunk.chunk_type);
        println!("| # | Name | Category |");
        println!("|---|------|----------|");
        for entry in &chunk_entries {
            println!(
                "| {} | {} | {} |",
                entry.index,
                entry.name,
                entry.category_display()
            );
        }
        println!();
    }
}

fn svd_import(
    port_pattern: &str,
    path: &std::path::Path,
    part: u8,
    index: Option<usize>,
    dry_run: bool,
) -> Result<()> {
    if !(1..=16).contains(&part) {
        bail!("part must be 1-16, got {part}");
    }
    let part_index = part - 1;

    let data = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let svd =
        SvdFile::parse(&data).with_context(|| format!("failed to parse {}", path.display()))?;

    // Find the SN Synth chunk (only supported type for now).
    let sns_chunk = svd
        .chunks
        .iter()
        .find(|c| c.chunk_type == ChunkType::SnSynthTone)
        .with_context(|| "no SN Synth Tone (SHPa) chunk in this SVD file")?;

    if sns_chunk.entries.is_empty() {
        println!("No SN Synth patches to import.");
        return Ok(());
    }

    // Determine which entries to import.
    let entries: Vec<(usize, &Vec<u8>)> = match index {
        Some(i) => {
            if i < 1 || i > sns_chunk.entries.len() {
                bail!("index {} out of range (1-{})", i, sns_chunk.entries.len());
            }
            vec![(i - 1, &sns_chunk.entries[i - 1])]
        }
        None => sns_chunk.entries.iter().enumerate().collect(),
    };

    println!(
        "Importing {} SN-S patch(es) to Part {} temporary area{}",
        entries.len(),
        part,
        if dry_run { " (dry run)" } else { "" }
    );

    // Open MIDI only if not dry-run.
    let mut conn = if dry_run {
        None
    } else {
        let (_conn_in, conn_out, _rx) = open_midi(port_pattern)?;
        Some((_conn_in, conn_out, _rx))
    };

    for (i, entry) in &entries {
        let sections = svd_to_sysex(entry, &SNS_TONE_SPEC)
            .with_context(|| format!("failed to decode entry {}", i + 1))?;

        // Extract tone name for display.
        let name: String = sections[0][..12]
            .iter()
            .map(|&b| {
                if (32..=127).contains(&b) {
                    b as char
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .trim_end()
            .to_string();

        let dt1s = sns_to_dt1s(DEVICE_ID, part_index, &sections);

        if dry_run {
            println!(
                "  {:>3}: {} ({} DT1 messages, {} bytes total)",
                i + 1,
                name,
                dt1s.len(),
                dt1s.iter().map(|m| m.len()).sum::<usize>()
            );
        } else {
            print!("  {:>3}: {} ... ", i + 1, name);
            let conn_out = &mut conn.as_mut().unwrap().1;
            for dt1 in &dt1s {
                conn_out.send(dt1).context("failed to send DT1")?;
                std::thread::sleep(Duration::from_millis(20));
            }
            println!("OK");

            // Small extra delay between patches.
            if entries.len() > 1 {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    if !dry_run {
        println!("Done. Patch is now in Part {}'s temporary area.", part);
        println!("Use the device to save it to user memory if desired.");
    }

    Ok(())
}

fn svd_validate(
    port_pattern: &str,
    timeout: Duration,
    path: &std::path::Path,
    part: u8,
    index: usize,
) -> Result<()> {
    if !(1..=16).contains(&part) {
        bail!("part must be 1-16, got {part}");
    }
    let part_index = part - 1;

    // Parse SVD and decode the entry.
    let data = std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let svd =
        SvdFile::parse(&data).with_context(|| format!("failed to parse {}", path.display()))?;
    let sns_chunk = svd
        .chunks
        .iter()
        .find(|c| c.chunk_type == ChunkType::SnSynthTone)
        .context("no SN Synth Tone chunk")?;
    if index < 1 || index > sns_chunk.entries.len() {
        bail!(
            "index {} out of range (1-{})",
            index,
            sns_chunk.entries.len()
        );
    }
    let sections = svd_to_sysex(&sns_chunk.entries[index - 1], &SNS_TONE_SPEC)
        .context("failed to decode SVD entry")?;

    // Sections are now: 0=Common, 1=MFX, 2=Partial1, 3=Partial2, 4=Partial3.
    let svd_common = &sections[0];
    let svd_mfx = &sections[1];

    let svd_name: String = svd_common[..12]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string();
    println!("SVD entry {index}: \"{svd_name}\"");

    // Open MIDI and read from the device.
    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    // Read each SN-S section from the device.
    let common_addr = sn_synth::sns_common_address(part_index);
    let common_size = sn_synth::SNS_COMMON_BLOCK_SIZE;
    println!("Reading SN-S Common from Part {} ...", part);
    let dev_common = request_data(&mut conn_out, &rx, &common_addr, &common_size, timeout)
        .context("failed to read Common")?;

    let dev_name: String = dev_common[..12]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string();
    println!("Device Part {part}: \"{dev_name}\"");

    if svd_name != dev_name {
        bail!(
            "Tone name mismatch: SVD=\"{svd_name}\" vs Device=\"{dev_name}\". Make sure the correct tone is loaded on Part {part}."
        );
    }

    // Compare Common.
    let mut mismatches = 0;
    print!("Common ({} bytes): ", svd_common.len());
    let common_ok = compare_sysex("Common", svd_common, &dev_common, &mut mismatches);
    println!("{}", if common_ok { "OK" } else { "MISMATCH" });

    // Read MFX in two chunks (header + params) since 273 bytes may exceed
    // the device's single-response limit.
    // MFX is at byte2-level offset 02 within the tone type block.
    let mfx_base = sn_synth::sns_common_address(part_index).offset([0x00, 0x00, 0x02, 0x00]);
    let mfx_hdr_size = mfx::MFX_HEADER_SIZE; // 0x11 bytes (type + sends + controls)
    println!("Reading MFX header ...");
    let mut dev_mfx = request_data(&mut conn_out, &rx, &mfx_base, &mfx_hdr_size, timeout)
        .context("failed to read MFX header")?;
    // Read params: 32 × 4 bytes starting at offset 0x11
    let mfx_params_addr = mfx_base.offset([0x00, 0x00, 0x00, 0x11]);
    let mfx_params_size = DataSize::new(0x00, 0x00, 0x01, 0x00); // 128 bytes (32×4)
    println!("Reading MFX params ...");
    let dev_mfx_params = request_data(
        &mut conn_out,
        &rx,
        &mfx_params_addr,
        &mfx_params_size,
        timeout,
    )
    .context("failed to read MFX params")?;
    dev_mfx.extend_from_slice(&dev_mfx_params);
    print!("MFX ({} bytes): ", svd_mfx.len());
    let mfx_ok = compare_sysex("MFX", svd_mfx, &dev_mfx, &mut mismatches);
    println!("{}", if mfx_ok { "OK" } else { "MISMATCH" });

    // Read Partials 1-3.
    for pi in 0..3u8 {
        let partial_addr = sn_synth::sns_partial_address(part_index, pi);
        let partial_size = sn_synth::SNS_PARTIAL_BLOCK_SIZE;
        println!("Reading Partial {} ...", pi + 1);
        let dev_partial = request_data(&mut conn_out, &rx, &partial_addr, &partial_size, timeout)
            .context(format!("failed to read Partial {}", pi + 1))?;
        let svd_partial = &sections[(pi + 2) as usize];
        print!("Partial {} ({} bytes): ", pi + 1, svd_partial.len());
        let p_ok = compare_sysex(
            &format!("Partial {}", pi + 1),
            svd_partial,
            &dev_partial,
            &mut mismatches,
        );
        println!("{}", if p_ok { "OK" } else { "MISMATCH" });
    }

    if mismatches == 0 {
        println!("\nValidation PASSED: all sections match.");
    } else {
        println!("\nValidation FAILED: {mismatches} byte(s) differ.");
    }

    Ok(())
}

/// Compare two SysEx byte vectors and report differences.
fn compare_sysex(label: &str, svd: &[u8], device: &[u8], mismatches: &mut usize) -> bool {
    let len = svd.len().min(device.len());
    let mut ok = true;
    if svd.len() != device.len() {
        eprintln!(
            "  {label}: length mismatch: SVD={} vs Device={}",
            svd.len(),
            device.len()
        );
        ok = false;
        *mismatches += svd.len().abs_diff(device.len());
    }
    for i in 0..len {
        if svd[i] != device[i] {
            if ok {
                // First mismatch for this section.
                ok = false;
            }
            eprintln!(
                "  {label}[0x{i:02X}]: SVD=0x{:02X} Device=0x{:02X}",
                svd[i], device[i]
            );
            *mismatches += 1;
        }
    }
    ok
}

fn svd_export(
    port_pattern: &str,
    timeout: Duration,
    path: &std::path::Path,
    part: u8,
) -> Result<()> {
    if !(1..=16).contains(&part) {
        bail!("part must be 1-16, got {part}");
    }
    let part_index = part - 1;

    let (_conn_in, mut conn_out, rx) = open_midi(port_pattern)?;

    // Read SN-S Common.
    let common_addr = sn_synth::sns_common_address(part_index);
    let common_size = sn_synth::SNS_COMMON_BLOCK_SIZE;
    println!("Reading SN-S Common from Part {} ...", part);
    let common = request_data(&mut conn_out, &rx, &common_addr, &common_size, timeout)
        .context("failed to read Common")?;

    let name: String = common[..12]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string();
    println!("Tone: \"{name}\"");

    // Read MFX (header + params).
    let mfx_base = sn_synth::sns_common_address(part_index).offset([0x00, 0x00, 0x02, 0x00]);
    println!("Reading MFX ...");
    let mut mfx_data = request_data(
        &mut conn_out,
        &rx,
        &mfx_base,
        &mfx::MFX_HEADER_SIZE,
        timeout,
    )
    .context("failed to read MFX header")?;
    let mfx_params_addr = mfx_base.offset([0x00, 0x00, 0x00, 0x11]);
    let mfx_params_size = DataSize::new(0x00, 0x00, 0x01, 0x00);
    let mfx_params = request_data(
        &mut conn_out,
        &rx,
        &mfx_params_addr,
        &mfx_params_size,
        timeout,
    )
    .context("failed to read MFX params")?;
    mfx_data.extend_from_slice(&mfx_params);

    // Read Partials 1-3.
    let mut partials = Vec::new();
    for pi in 0..3u8 {
        let addr = sn_synth::sns_partial_address(part_index, pi);
        println!("Reading Partial {} ...", pi + 1);
        let data = request_data(
            &mut conn_out,
            &rx,
            &addr,
            &sn_synth::SNS_PARTIAL_BLOCK_SIZE,
            timeout,
        )
        .context(format!("failed to read Partial {}", pi + 1))?;
        partials.push(data);
    }

    // Pack into SVD entry.
    let sections = vec![
        common,
        mfx_data,
        partials[0].clone(),
        partials[1].clone(),
        partials[2].clone(),
    ];
    let entry = sysex_to_svd(&sections, &SNS_TONE_SPEC);

    // Build SVD file with a single SN-S entry.
    let svd = SvdFile {
        chunks: vec![
            SvdChunk {
                chunk_type: ChunkType::StudioSet,
                entry_size: 1068,
                entries: vec![],
            },
            SvdChunk {
                chunk_type: ChunkType::PcmSynthTone,
                entry_size: 590,
                entries: vec![],
            },
            SvdChunk {
                chunk_type: ChunkType::PcmDrumKit,
                entry_size: 10890,
                entries: vec![],
            },
            SvdChunk {
                chunk_type: ChunkType::SnSynthTone,
                entry_size: 280,
                entries: vec![entry],
            },
            SvdChunk {
                chunk_type: ChunkType::SnAcousticTone,
                entry_size: 138,
                entries: vec![],
            },
            SvdChunk {
                chunk_type: ChunkType::SnDrumKit,
                entry_size: 1006,
                entries: vec![],
            },
        ],
    };

    let data = svd.write();
    std::fs::write(path, &data).with_context(|| format!("failed to write {}", path.display()))?;

    println!(
        "Exported \"{}\" to {} ({} bytes)",
        name,
        path.display(),
        data.len()
    );
    Ok(())
}
