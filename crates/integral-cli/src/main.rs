use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use integral_core::address::{Address, DataSize};
use integral_core::params;
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
    /// Dump all incoming MIDI messages (for debugging).
    Monitor {
        #[arg(long, default_value = "Integra")]
        port: String,
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
        Cli::Monitor { port } => monitor(&port),
    }
}
