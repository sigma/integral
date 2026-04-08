use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use integral_core::sysex;
use midir::{MidiInput, MidiOutput};

/// Integral — CLI tools for the Roland INTEGRA-7.
#[derive(Parser)]
#[command(version)]
enum Cli {
    /// Ping the INTEGRA-7 via SysEx Identity Request.
    Ping {
        /// MIDI port name or substring to match (case-insensitive).
        #[arg(long, default_value = "Integra")]
        port: String,

        /// Timeout in seconds.
        #[arg(long, default_value_t = 2.0)]
        timeout: f64,

        /// Suppress output; exit code only.
        #[arg(long)]
        quiet: bool,
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

fn ping(port_pattern: &str, timeout: Duration, quiet: bool) -> Result<()> {
    let midi_out = MidiOutput::new("integral-ping-out").context("failed to create MIDI output")?;
    let midi_in = MidiInput::new("integral-ping-in").context("failed to create MIDI input")?;

    let out_port = find_port_by_name(&midi_out, port_pattern)
        .with_context(|| format!("no MIDI output port matching '{port_pattern}'"))?;
    let in_port = find_port_by_name(&midi_in, port_pattern)
        .with_context(|| format!("no MIDI input port matching '{port_pattern}'"))?;

    let port_name = midi_out
        .port_name(&out_port)
        .unwrap_or_else(|_| port_pattern.to_string());

    if !quiet {
        eprint!("Pinging INTEGRA-7 on '{port_name}'... ");
    }

    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let _conn_in = midi_in
        .connect(
            &in_port,
            "integral-ping-in",
            move |_timestamp, message, _| {
                let _ = tx.send(message.to_vec());
            },
            (),
        )
        .context("failed to connect MIDI input")?;

    let mut conn_out = midi_out
        .connect(&out_port, "integral-ping-out")
        .context("failed to connect MIDI output")?;

    conn_out
        .send(&sysex::identity_request())
        .context("failed to send identity request")?;

    let deadline = timeout;
    loop {
        match rx.recv_timeout(deadline) {
            Ok(data) => {
                match sysex::parse_identity_reply(&data) {
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
                    Err(_) => continue, // not an identity reply, keep listening
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !quiet {
                    eprintln!("FAIL: No response (device off or wrong port?)");
                }
                bail!("no response from device");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("MIDI input disconnected");
            }
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
    }
}
