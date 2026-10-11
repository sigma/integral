//! Raw SysEx commands: raw-read, raw-rq1, raw-send, raw-hex.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use integral_core::librarian::reader::BlockData;
use integral_core::sysex;

use crate::hex::{format_hex, parse_hex_addr, parse_hex_bytes, parse_hex_size, parse_read_size};
use crate::midi;

/// Read raw bytes from a SysEx address.
///
/// Collects every DT1 packet of the reply within `timeout`, places each by
/// its address, and fails if any byte is missing.
pub fn raw_read(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    addr_hex: &str,
    size_arg: &str,
) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let size = parse_read_size(size_arg)?;
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    let read = midi::read_block(&mut conn_out, &rx, device_id, &addr, &size, timeout, true)?;
    let (data, missing) = match &read {
        BlockData::Complete(data) => (data, &[][..]),
        BlockData::Gap { data, missing } => (data, &missing[..]),
        BlockData::Timeout => bail!("no reply at {addr}"),
    };

    println!("Hex:   {}", format_hex(data, " "));
    println!("ASCII: {}", ascii(data));

    if !missing.is_empty() {
        bail!(
            "incomplete reply: byte ranges {} missing (shown as 00)",
            midi::format_ranges(missing)
        );
    }
    Ok(())
}

/// Send RQ1 and capture all responses (for multi-response queries).
pub fn raw_rq1_multi(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    addr_hex: &str,
    size_hex: &str,
) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let size = parse_hex_size(size_hex)?;
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    let rq1 = sysex::build_rq1(device_id, &addr, &size);
    eprintln!("Sending RQ1: addr={}, size={}", addr, size);
    conn_out.send(&rq1).context("failed to send RQ1")?;

    print_replies(&rx, timeout)
}

/// Send raw hex bytes as SysEx and capture all responses.
pub fn send_raw_hex(port_pattern: &str, timeout: Duration, hex: &str) -> Result<()> {
    let bytes = parse_hex_bytes(hex)?;
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    eprintln!("Sending raw: {}", format_hex(&bytes, " "));
    conn_out.send(&bytes).context("send failed")?;

    print_replies(&rx, timeout)
}

/// Send a raw DT1 message and capture responses.
pub fn raw_send(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    addr_hex: &str,
    data_hex: &str,
) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let data = parse_hex_bytes(data_hex)?;
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    let dt1 = sysex::build_dt1(device_id, &addr, &data);
    eprintln!("Sending DT1: addr={}, data={}", addr, data_hex);
    conn_out.send(&dt1).context("failed to send DT1")?;

    print_replies(&rx, timeout)
}

/// Print every message received within `timeout`: DT1s by address, with
/// their data as hex and ASCII; anything else as raw hex.
fn print_replies(rx: &mpsc::Receiver<Vec<u8>>, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut count = 0;
    loop {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(msg) => {
                count += 1;
                match sysex::parse_dt1(&msg) {
                    Ok(dt1) => println!(
                        "[{count}] addr={} ({} bytes) {} |{}|",
                        dt1.address,
                        dt1.data.len(),
                        format_hex(&dt1.data, " "),
                        ascii(&dt1.data)
                    ),
                    Err(_) => println!(
                        "[{count}] raw ({} bytes): {}",
                        msg.len(),
                        format_hex(&msg, " ")
                    ),
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("disconnected"),
        }
    }
    eprintln!("Received {count} messages");
    Ok(())
}

/// `data` as ASCII, with `.` for every non-printable byte.
fn ascii(data: &[u8]) -> String {
    data.iter()
        .map(|&b| {
            if b == b' ' || b.is_ascii_graphic() {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_masks_non_printable_bytes() {
        assert_eq!(ascii(b"Hi there\x00\x7F\x1F~"), "Hi there...~");
    }
}
