//! Raw SysEx commands: raw-read, raw-rq1, raw-send, raw-hex.

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::sysex;

use crate::hex::{parse_hex_addr, parse_hex_bytes, parse_hex_size};
use crate::midi;

/// Read raw bytes from a SysEx address.
pub fn raw_read(
    port_pattern: &str,
    timeout: Duration,
    device_id: u8,
    addr_hex: &str,
    size_hex: &str,
) -> Result<()> {
    let addr = parse_hex_addr(addr_hex)?;
    let size = parse_hex_size(size_hex)?;
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

    let data = midi::request_data(&mut conn_out, &rx, device_id, &addr, &size, timeout)?;

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

/// Send raw hex bytes as SysEx and capture all responses.
pub fn send_raw_hex(port_pattern: &str, timeout: Duration, hex: &str) -> Result<()> {
    let bytes = parse_hex_bytes(hex)?;
    let (_conn_in, mut conn_out, rx) = midi::open_midi(port_pattern)?;

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
