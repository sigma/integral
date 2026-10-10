//! Monitor command: dump incoming MIDI messages, optionally bridging two ports.

use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use integral_core::sysex;
use midir::MidiOutputConnection;

use crate::hex::format_hex;
use crate::midi;

/// Monitor all incoming MIDI messages (blocks until Ctrl+C).
///
/// With `bridge`, also forward everything between `port_pattern` and
/// `bridge` in both directions, labelling each message with its direction.
/// This puts the CLI between another app (e.g. Roland's iPad editor on a
/// network MIDI session) and the device, so both sides of a conversation
/// can be captured.
pub fn monitor(port_pattern: &str, bridge: Option<&str>) -> Result<()> {
    let (_conn_in, conn_out, rx) = midi::open_midi(port_pattern)?;
    let start = Instant::now();
    let Some(bridge) = bridge else {
        eprintln!("Monitoring incoming MIDI messages (Ctrl+C to stop)...");
        return relay(&rx, None, "", start);
    };

    let (_bridge_in, bridge_out, bridge_rx) = midi::open_midi(bridge)?;
    eprintln!("Bridging '{port_pattern}' <-> '{bridge}' (Ctrl+C to stop)...");
    let to_port = format!("{bridge} -> {port_pattern}: ");
    let back = thread::spawn(move || relay(&bridge_rx, Some(conn_out), &to_port, start));
    relay(
        &rx,
        Some(bridge_out),
        &format!("{port_pattern} -> {bridge}: "),
        start,
    )?;
    back.join().expect("bridge thread panicked")
}

/// Print each message from `rx`, forwarding it to `out` when given.
fn relay(
    rx: &Receiver<Vec<u8>>,
    mut out: Option<MidiOutputConnection>,
    label: &str,
    start: Instant,
) -> Result<()> {
    loop {
        let Ok(data) = rx.recv() else {
            bail!("MIDI input disconnected");
        };
        if let Some(out) = out.as_mut() {
            out.send(&data).context("failed to forward MIDI")?;
        }
        let ms = start.elapsed().as_millis();
        println!("{ms:>8} ms {label}{}", describe(&data));
    }
}

/// Format a MIDI message as hex, with a DT1 decode when it parses as one.
fn describe(data: &[u8]) -> String {
    let mut s = format!("[{} bytes] {}", data.len(), format_hex(data, " "));
    if let Ok(dt1) = sysex::parse_dt1(data) {
        s += &format!(
            "  → DT1 addr={} data=[{}]",
            dt1.address,
            format_hex(&dt1.data, ", ")
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_dt1() {
        let msg = [
            0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x12, 0x0F, 0x00, 0x20, 0x00, 0x01, 0x50, 0xF7,
        ];
        assert_eq!(
            describe(&msg),
            "[14 bytes] F0 41 10 00 00 64 12 0F 00 20 00 01 50 F7  → DT1 addr=0F 00 20 00 data=[01]"
        );
    }

    #[test]
    fn describes_non_sysex() {
        assert_eq!(describe(&[0x90, 0x3C, 0x64]), "[3 bytes] 90 3C 64");
    }
}
