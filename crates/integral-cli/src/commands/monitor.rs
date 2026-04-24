//! Monitor command: dump all incoming MIDI messages.

use anyhow::{Result, bail};
use integral_core::sysex;

use crate::midi;

/// Monitor all incoming MIDI messages (blocks until Ctrl+C).
pub fn monitor(port_pattern: &str) -> Result<()> {
    let (_conn_in, _conn_out, rx) = midi::open_midi(port_pattern)?;
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
