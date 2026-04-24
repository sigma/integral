//! Ping command: verify INTEGRA-7 connectivity via Identity Request.

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::sysex;

use crate::midi;

/// Ping the device and return the detected device ID on success.
pub fn ping(port_pattern: &str, timeout: Duration, quiet: bool) -> Result<u8> {
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

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
                    return Ok(identity.device_id);
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
